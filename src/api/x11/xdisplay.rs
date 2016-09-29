use std::ptr;
use std::fmt;
use std::error::Error;
use std::ffi::CString;
use std::sync::Mutex;

use libc;
use winit;

use super::ffi;
use api::egl::ffi::egl::Egl;
use api::dlopen;

/// A connection to an X server.
pub struct XConnection {
    pub w: winit::api::x11::XConnection, // TODO: rename
    pub glx: Option<ffi::glx::Glx>,
    pub egl: Option<Egl>,
}

unsafe impl Send for XConnection {}
unsafe impl Sync for XConnection {}

pub type XErrorHandler = Option<unsafe extern fn(*mut ffi::Display, *mut ffi::XErrorEvent) -> libc::c_int>;

impl XConnection {
    pub fn new(error_handler: XErrorHandler) -> Result<XConnection, XNotSupported> {
        // TODO: use something safer than raw "dlopen"
        let glx = {
            let mut libglx = unsafe { dlopen::dlopen(b"libGL.so.1\0".as_ptr() as *const _, dlopen::RTLD_NOW) };
            if libglx.is_null() {
                libglx = unsafe { dlopen::dlopen(b"libGL.so\0".as_ptr() as *const _, dlopen::RTLD_NOW) };
            }

            if libglx.is_null() {
                None
            } else {
                Some(ffi::glx::Glx::load_with(|sym| {
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

        // TODO: использовать то же, что и в platfrom/linux/..
        /*
        unsafe extern "C" fn x_error_callback(
            _dpy: *mut winit::api::x11::ffi::Display,
            _event: *mut winit::api::x11::ffi::XErrorEvent,
        ) -> libc::c_int {
            unimplemented!();
        }
        let w = winit::api::x11::XConnection::new(Some(x_error_callback)).unwrap();
        */

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
