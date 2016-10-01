use std::ffi::CString;

use winit;

use super::ffi::glx::Glx;
use api::egl::ffi::egl::Egl;
use api::dlopen;

pub use winit::api::x11::XError;
pub use winit::api::x11::XNotSupported;

// TODO: rename
pub struct GlenOrGlenda {
    pub glx: Option<Glx>,
    pub egl: Option<Egl>,
}

impl GlenOrGlenda {
    pub fn new() -> GlenOrGlenda {
        // TODO: use something safer than raw "dlopen"
        let glx = {
            let mut libglx = unsafe { dlopen::dlopen(b"libGL.so.1\0".as_ptr() as *const _, dlopen::RTLD_NOW) };
            if libglx.is_null() {
                libglx = unsafe { dlopen::dlopen(b"libGL.so\0".as_ptr() as *const _, dlopen::RTLD_NOW) };
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
            let mut libegl = unsafe { dlopen::dlopen(b"libEGL.so.1\0".as_ptr() as *const _, dlopen::RTLD_NOW) };
            if libegl.is_null() {
                libegl = unsafe { dlopen::dlopen(b"libEGL.so\0".as_ptr() as *const _, dlopen::RTLD_NOW) };
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
        GlenOrGlenda {
            glx: glx,
            egl: egl,
        }
    }
}

/// A connection to an X server.
pub struct XConnection {
    pub w: winit::api::x11::XConnection, // TODO: rename
}

unsafe impl Send for XConnection {}
unsafe impl Sync for XConnection {}

impl XConnection {
    pub fn new() -> Result<XConnection, XNotSupported> {
        // TODO: use something safer than raw "dlopen"
        Ok(XConnection {
            w: winit::api::x11::XConnection::new(None).unwrap(),
        })
    }

    /// Checks whether an error has been triggered by the previous function calls.
    #[inline]
    pub fn check_errors(&self) -> Result<(), XError> {
        let error = self.w.latest_error.lock().unwrap().take();

        if let Some(error) = error {
            Err(error)
        } else {
            Ok(())
        }
    }

    /// Ignores any previous error.
    #[inline]
    pub fn ignore_error(&self) {
        *self.w.latest_error.lock().unwrap() = None;
    }
}
