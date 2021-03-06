#![cfg(target_os = "windows")]

pub use api::win32::{self, MonitorId, WindowProxy};

pub use winit::{get_available_monitors, get_primary_monitor};

use Api;
use ContextError;
use CreationError;
use PixelFormat;
use PixelFormatRequirements;
use GlAttributes;
use GlContext;
use WindowAttributes;

use winit;

use api::egl::ffi::egl::Egl;
use api::egl;
use api::egl::Context as EglContext;

use std::ffi::CString;
use std::ops::{Deref, DerefMut};
use kernel32;

/// Stupid wrapper because `*const libc::c_void` doesn't implement `Sync`.
struct EglWrapper(Egl);
unsafe impl Sync for EglWrapper {}

lazy_static! {
    // An EGL implementation available on the system.
    static ref EGL: Option<EglWrapper> = {
        // the ATI drivers provide an EGL implementation in their DLLs
        let dll_name = if cfg!(target_pointer_width = "64") {
            b"atio6axx.dll\0"
        } else {
            b"atioglxx.dll\0"
        };

        let dll = unsafe { kernel32::LoadLibraryA(dll_name.as_ptr() as *const _) };

        if !dll.is_null() {
            let egl = Egl::load_with(|name| {
                let name = CString::new(name).unwrap();
                unsafe { kernel32::GetProcAddress(dll, name.as_ptr()) as *const _ }
            });

            Some(EglWrapper(egl))

        } else {
            None
        }
    };
}

#[derive(Clone, Default)]
pub struct PlatformSpecificWindowBuilderAttributes;
#[derive(Clone, Default)]
pub struct PlatformSpecificHeadlessBuilderAttributes;

/// The Win32 implementation of the main `Window` object.
pub struct Window(win32::Window);

impl Window {
    /// See the docs in the crate root file.
    #[inline]
    pub fn new(
        window: &WindowAttributes,
        pf_reqs: &PixelFormatRequirements,
        opengl: &GlAttributes<&Window>,
        _: &PlatformSpecificWindowBuilderAttributes,
        winit_window: &winit::Window,
    ) -> Result<Window, CreationError> {
        win32::Window::new(
            window,
            pf_reqs,
            &opengl.clone().map_sharing(|w| &w.0),
            EGL.as_ref().map(|w| &w.0),
            winit_window,
        ).map(|w| Window(w))
    }
}

impl Deref for Window {
    type Target = win32::Window;

    #[inline]
    fn deref(&self) -> &win32::Window {
        &self.0
    }
}

impl DerefMut for Window {
    #[inline]
    fn deref_mut(&mut self) -> &mut win32::Window {
        &mut self.0
    }
}

///
pub enum HeadlessContext {
    /// A regular window, but invisible.
    HiddenWindow(win32::Window),
    /// An EGL pbuffer.
    EglPbuffer(EglContext),
}

impl HeadlessContext {
    pub fn new(dimensions: (u32, u32), pf_reqs: &PixelFormatRequirements,
               opengl: &GlAttributes<&HeadlessContext>,
               _: &PlatformSpecificHeadlessBuilderAttributes,
               /*winit_window: &winit::Window*/) // TODO OZKRIFF
               -> Result<HeadlessContext, CreationError>
    {
        // if EGL is available, we try using EGL first
        // if EGL returns an error, we try the hidden window method
        if let &Some(ref egl) = &*EGL {
            let context = EglContext::new(egl.0.clone(), pf_reqs, &opengl.clone().map_sharing(|_| unimplemented!()),       // TODO:
                                          egl::NativeDisplay::Other(None))
                                .and_then(|prototype| prototype.finish_pbuffer(dimensions))
                                .map(|ctxt| HeadlessContext::EglPbuffer(ctxt));

            if let Ok(context) = context {
                return Ok(context);
            }
        }

        unimplemented!(); // TODO OZKRIFF
        // эээ, наверное нет смысла для хедлес окна создавать винит-окно.
        // но оно мне нужно, что бы вызвать у него метод platform_window =\
        // иначе что я буду использовать при конструировании контекста?
        /*
        let window = try!(win32::Window::new(&WindowAttributes { visible: false, .. Default::default() },
                                             pf_reqs, &opengl.clone().map_sharing(|_| unimplemented!()),            //TODO:
                                             EGL.as_ref().map(|w| &w.0),
                                             winit_window));
        Ok(HeadlessContext::HiddenWindow(window))
        */
    }
}

impl GlContext for HeadlessContext {
    #[inline]
    unsafe fn make_current(&self) -> Result<(), ContextError> {
        match self {
            &HeadlessContext::HiddenWindow(ref ctxt) => ctxt.make_current(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.make_current(),
        }
    }

    #[inline]
    fn is_current(&self) -> bool {
        match self {
            &HeadlessContext::HiddenWindow(ref ctxt) => ctxt.is_current(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.is_current(),
        }
    }

    #[inline]
    fn get_proc_address(&self, addr: &str) -> *const () {
        match self {
            &HeadlessContext::HiddenWindow(ref ctxt) => ctxt.get_proc_address(addr),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.get_proc_address(addr),
        }
    }

    #[inline]
    fn swap_buffers(&self) -> Result<(), ContextError> {
        match self {
            &HeadlessContext::HiddenWindow(ref ctxt) => ctxt.swap_buffers(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.swap_buffers(),
        }
    }

    #[inline]
    fn get_api(&self) -> Api {
        match self {
            &HeadlessContext::HiddenWindow(ref ctxt) => ctxt.get_api(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.get_api(),
        }
    }

    #[inline]
    fn get_pixel_format(&self) -> PixelFormat {
        match self {
            &HeadlessContext::HiddenWindow(ref ctxt) => ctxt.get_pixel_format(),
            &HeadlessContext::EglPbuffer(ref ctxt) => ctxt.get_pixel_format(),
        }
    }
}
