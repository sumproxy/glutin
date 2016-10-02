use std::sync::Arc;

use winit;

use ContextError;
use CreationError;
use GlAttributes;
use GlContext;
use PixelFormat;
use PixelFormatRequirements;
use WindowAttributes;
use libc;

use api::wayland;
use api::x11::{self, XConnection, XNotSupported, XError};

#[derive(Clone, Default)]
pub struct PlatformSpecificWindowBuilderAttributes;

enum Backend {
    X(Arc<XConnection>),
    Wayland,
    Error(XNotSupported),
}

// TODO: OZKRIFF: надо бы это тоже грохнуть, по идее нужная информация вся уже есть у winit
lazy_static!(
    static ref BACKEND: Backend = {
        // Wayland backend is not production-ready yet so we disable it
        if wayland::is_available() {
            Backend::Wayland
        } else {
            match XConnection::new(Some(x_error_callback)) {
                Ok(x) => Backend::X(Arc::new(x)),
                Err(e) => Backend::Error(e),
            }
        }
    };
);

pub enum Window {
    #[doc(hidden)]
    X(x11::Window),
    #[doc(hidden)]
    Wayland(wayland::Window)
}

pub use winit::platform::{MonitorId, get_available_monitors, get_primary_monitor};

impl Window {
    #[inline]
    pub fn new(
        window: &WindowAttributes, // вот это надо бы убрать
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Window>,
        _: &PlatformSpecificWindowBuilderAttributes, // и это, наверное, тоже убрать
        ozkriff_window: &winit::Window,
    ) -> Result<Window, CreationError> {
        match *BACKEND {
            Backend::Wayland => {
                let opengl = opengl.clone().map_sharing(|w| match w {
                    &Window::Wayland(ref w) => w,
                    _ => panic!()       // TODO: return an error
                });

                wayland::Window::new(window, pf_reqs, &opengl).map(Window::Wayland)
            },

            Backend::X(ref connec) => {
                let opengl = opengl.clone().map_sharing(|w| match w {
                    &Window::X(ref w) => w,
                    _ => panic!()       // TODO: return an error
                });
                x11::Window::new(
                    connec,
                    window,
                    pf_reqs,
                    &opengl,
                    ozkriff_window,
                ).map(Window::X)
            },

            Backend::Error(ref error) => Err(CreationError::NoBackendAvailable(Box::new(error.clone())))
        }
    }
}

impl GlContext for Window {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        match self {
            &Window::X(ref w) => w.make_current(),
            &Window::Wayland(ref w) => w.make_current()
        }
    }

    #[inline]
    fn is_current(&self) -> bool {
        match self {
            &Window::X(ref w) => w.is_current(),
            &Window::Wayland(ref w) => w.is_current()
        }
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        match self {
            &Window::X(ref w) => w.get_proc_address(addr),
            &Window::Wayland(ref w) => w.get_proc_address(addr)
        }
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        match self {
            &Window::X(ref w) => w.swap_buffers(),
            &Window::Wayland(ref w) => w.swap_buffers()
        }
    }

    #[inline]
    fn get_api(&self) -> ::Api {
        match self {
            &Window::X(ref w) => w.get_api(),
            &Window::Wayland(ref w) => w.get_api()
        }
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        match self {
            &Window::X(ref w) => w.get_pixel_format(),
            &Window::Wayland(ref w) => w.get_pixel_format()
        }
    }
}

unsafe extern "C" fn x_error_callback(dpy: *mut x11::ffi::Display, event: *mut x11::ffi::XErrorEvent)
                                      -> libc::c_int
{
    use std::ffi::CStr;

    if let Backend::X(ref x) = *BACKEND {
        let mut buff: Vec<u8> = Vec::with_capacity(1024);
        (x.xlib.XGetErrorText)(dpy, (*event).error_code as i32, buff.as_mut_ptr() as *mut libc::c_char, buff.capacity() as i32);
        let description = CStr::from_ptr(buff.as_mut_ptr() as *const libc::c_char).to_string_lossy();

        let error = XError {
            description: description.into_owned(),
            error_code: (*event).error_code,
            request_code: (*event).request_code,
            minor_code: (*event).minor_code,
        };

        *x.latest_error.lock().unwrap() = Some(error);
    }

    0
}
