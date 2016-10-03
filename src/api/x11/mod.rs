#![cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "openbsd"))]

pub use winit::api::x11::{XError, XNotSupported, XConnection};

pub mod ffi;

use CreationError;
use libc;
use std::borrow::Borrow;
use std::{mem, ptr};
use std::sync::{Arc};

use winit;

use Api;
use ContextError;
use GlAttributes;
use GlContext;
use GlRequest;
use PixelFormat;
use PixelFormatRequirements;

use std::ffi::CString;

use api::glx::Context as GlxContext;
use api::egl;
use api::egl::Context as EglContext;
use api::x11::ffi::glx::Glx;
use api::egl::ffi::egl::Egl;
use api::dlopen;

struct GlxOrEgl {
    glx: Option<Glx>,
    egl: Option<Egl>,
}

impl GlxOrEgl {
    fn new() -> GlxOrEgl {
        // TODO: use something safer than raw "dlopen"
        let glx = {
            let mut libglx = unsafe {
                dlopen::dlopen(b"libGL.so.1\0".as_ptr() as *const _, dlopen::RTLD_NOW)
            };
            if libglx.is_null() {
                libglx = unsafe {
                    dlopen::dlopen(b"libGL.so\0".as_ptr() as *const _, dlopen::RTLD_NOW)
                };
            }
            if libglx.is_null() {
                None
            } else {
                Some(Glx::load_with(|sym| {
                    let sym = CString::new(sym).unwrap();
                    unsafe { dlopen::dlsym(libglx, sym.as_ptr()) }
                }))
            }
        };
        // TODO: use something safer than raw "dlopen"
        let egl = {
            let mut libegl = unsafe {
                dlopen::dlopen(b"libEGL.so.1\0".as_ptr() as *const _, dlopen::RTLD_NOW)
            };
            if libegl.is_null() {
                libegl = unsafe {
                    dlopen::dlopen(b"libEGL.so\0".as_ptr() as *const _, dlopen::RTLD_NOW)
                };
            }
            if libegl.is_null() {
                None
            } else {
                Some(Egl::load_with(|sym| {
                    let sym = CString::new(sym).unwrap();
                    unsafe { dlopen::dlsym(libegl, sym.as_ptr()) }
                }))
            }
        };
        GlxOrEgl {
            glx: glx,
            egl: egl,
        }
    }
}

enum Context {
    Glx(GlxContext),
    Egl(EglContext),
    None,
}

pub struct Window {
    display: Arc<XConnection>, // нужен, что бы кое-какие функции для той же карты цветов вызвать
    colormap: ffi::Colormap,
    context: Context,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            // we don't call MakeCurrent(0, 0) because we are not sure that the context
            // is still the current one
            self.context = Context::None;

            (self.display.xlib.XFreeColormap)(self.display.display, self.colormap);
        }
    }
}

impl Window {
    pub fn new(
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Window>,
        ozkriff_window: &winit::Window,
    ) -> Result<Window, CreationError> {
        let ozkriff_x11: &winit::api::x11::XWindow = match ozkriff_window.window {
            winit::platform::Window::X(ref w) => w.x.borrow(),
            winit::platform::Window::Wayland(_) => unimplemented!(),
        };
        let display = &ozkriff_x11.display;
        let screen_id = ozkriff_x11.screen_id;

        // start the context building process
        enum Prototype<'a> {
            Glx(::api::glx::ContextPrototype<'a>),
            Egl(::api::egl::ContextPrototype<'a>),
        }
        let builder_clone_opengl_glx = opengl.clone().map_sharing(|_| unimplemented!());      // FIXME:
        let builder_clone_opengl_egl = opengl.clone().map_sharing(|_| unimplemented!());      // FIXME:
        let backend = GlxOrEgl::new();
        let context = match opengl.version {
            GlRequest::Latest | GlRequest::Specific(Api::OpenGl, _) | GlRequest::GlThenGles { .. } => {
                // GLX should be preferred over EGL, otherwise crashes may occur
                // on X11 – issue #314
                if let Some(ref glx) = backend.glx {
                    Prototype::Glx(try!(GlxContext::new(
                        glx.clone(),
                        &display.xlib,
                        pf_reqs,
                        &builder_clone_opengl_glx,
                        display.display,
                        screen_id,
                        ozkriff_window,
                    )))
                } else if let Some(ref egl) = backend.egl {
                    Prototype::Egl(try!(EglContext::new(
                            egl.clone(),
                        pf_reqs,
                        &builder_clone_opengl_egl,
                        egl::NativeDisplay::X11(Some(display.display as *const _)),
                    )))
                } else {
                    return Err(CreationError::NotSupported);
                }
            },
            GlRequest::Specific(Api::OpenGlEs, _) => {
                if let Some(ref egl) = backend.egl {
                    Prototype::Egl(try!(EglContext::new(
                        egl.clone(),
                        pf_reqs,
                        &builder_clone_opengl_egl,
                        egl::NativeDisplay::X11(Some(display.display as *const _)),
                    )))
                } else {
                    return Err(CreationError::NotSupported);
                }
            },
            GlRequest::Specific(_, _) => {
                return Err(CreationError::NotSupported);
            },
        };

        // getting the `visual_infos` (a struct that contains information about the visual to use)
        let visual_infos = match context {
            Prototype::Glx(ref p) => p.get_visual_infos().clone(),
            Prototype::Egl(ref p) => {
                unsafe {
                    let mut template: ffi::XVisualInfo = mem::zeroed();
                    template.visualid = p.get_native_visual_id() as ffi::VisualID;

                    let mut num_visuals = 0;
                    let vi = (display.xlib.XGetVisualInfo)(display.display, ffi::VisualIDMask,
                                                           &mut template, &mut num_visuals);
                    display.check_errors().expect("Failed to call XGetVisualInfo");
                    assert!(!vi.is_null());
                    assert!(num_visuals == 1);

                    let vi_copy = ptr::read(vi as *const _);
                    (display.xlib.XFree)(vi as *mut _);
                    vi_copy
                }
            },
        };

        let window = ozkriff_x11.window;

        // finish creating the OpenGL context
        let context = match context {
            Prototype::Glx(ctxt) => {
                Context::Glx(try!(ctxt.finish(window)))
            },
            Prototype::Egl(ctxt) => {
                Context::Egl(try!(ctxt.finish(window as *const libc::c_void)))
            },
        };

        // getting the root window
        let root = unsafe { (display.xlib.XDefaultRootWindow)(display.display) };
        display.check_errors().expect("Failed to get root window");

        // creating the color map
        let cmap = unsafe {
            let cmap = (display.xlib.XCreateColormap)(display.display, root,
                                                      visual_infos.visual as *mut _,
                                                      ffi::AllocNone);
            display.check_errors().expect("Failed to call XCreateColormap");
            cmap
        };

        Ok(Window {
            display: display.clone(),
            context: context,
            colormap: cmap,
        })
    }
}

impl GlContext for Window {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        match self.context {
            Context::Glx(ref ctxt) => ctxt.make_current(),
            Context::Egl(ref ctxt) => ctxt.make_current(),
            Context::None => Ok(())
        }
    }

    #[inline]
    fn is_current(&self) -> bool {
        match self.context {
            Context::Glx(ref ctxt) => ctxt.is_current(),
            Context::Egl(ref ctxt) => ctxt.is_current(),
            Context::None => panic!()
        }
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        match self.context {
            Context::Glx(ref ctxt) => ctxt.get_proc_address(addr),
            Context::Egl(ref ctxt) => ctxt.get_proc_address(addr),
            Context::None => ptr::null()
        }
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        match self.context {
            Context::Glx(ref ctxt) => ctxt.swap_buffers(),
            Context::Egl(ref ctxt) => ctxt.swap_buffers(),
            Context::None => Ok(())
        }
    }

    #[inline]
    fn get_api(&self) -> Api {
        match self.context {
            Context::Glx(ref ctxt) => ctxt.get_api(),
            Context::Egl(ref ctxt) => ctxt.get_api(),
            Context::None => panic!()
        }
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        match self.context {
            Context::Glx(ref ctxt) => ctxt.get_pixel_format(),
            Context::Egl(ref ctxt) => ctxt.get_pixel_format(),
            Context::None => panic!()
        }
    }
}
