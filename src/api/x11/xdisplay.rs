use std::ffi::CString;

use winit;

use super::ffi::glx::Glx;
use api::egl::ffi::egl::Egl;
use api::dlopen;

/// A connection to an X server.
pub struct XConnection {
    pub w: winit::api::x11::XConnection, // TODO: rename
    pub glx: Option<Glx>,
    pub egl: Option<Egl>,
}

unsafe impl Send for XConnection {}
unsafe impl Sync for XConnection {}

impl XConnection {
    pub fn new() -> Result<XConnection, XNotSupported> {
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
        let w = winit::api::x11::XConnection::new(None).unwrap();
        Ok(XConnection {
            w: w,
            glx: glx,
            egl: egl,
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

pub use winit::api::x11::XError;
pub use winit::api::x11::XNotSupported;
