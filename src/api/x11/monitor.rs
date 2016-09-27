use std::collections::VecDeque;
use std::sync::Arc;

use super::XConnection;
use native_monitor::NativeMonitorId;

// TODO: это вот точно надо заменить к фигам на winitовский тип!

#[derive(Clone)]
pub struct MonitorId(pub Arc<XConnection>, pub u32);

pub fn get_available_monitors(x: &Arc<XConnection>) -> VecDeque<MonitorId> {
    let nb_monitors = unsafe { (x.w.xlib.XScreenCount)(x.w.display) };
    x.check_errors().expect("Failed to call XScreenCount");

    let mut monitors = VecDeque::new();
    monitors.extend((0 .. nb_monitors).map(|i| MonitorId(x.clone(), i as u32)));
    monitors
}

#[inline]
pub fn get_primary_monitor(x: &Arc<XConnection>) -> MonitorId {
    let primary_monitor = unsafe { (x.w.xlib.XDefaultScreen)(x.w.display) };
    x.check_errors().expect("Failed to call XDefaultScreen");
    MonitorId(x.clone(), primary_monitor as u32)
}

impl MonitorId {
    pub fn get_name(&self) -> Option<String> {
        let MonitorId(_, screen_num) = *self;
        Some(format!("Monitor #{}", screen_num))
    }

    #[inline]
    pub fn get_native_identifier(&self) -> NativeMonitorId {
        NativeMonitorId::Numeric(self.1)
    }

    pub fn get_dimensions(&self) -> (u32, u32) {
        let screen = unsafe { (self.0.w.xlib.XScreenOfDisplay)(self.0.w.display, self.1 as i32) };
        let width = unsafe { (self.0.w.xlib.XWidthOfScreen)(screen) };
        let height = unsafe { (self.0.w.xlib.XHeightOfScreen)(screen) };
        self.0.check_errors().expect("Failed to get monitor dimensions");
        (width as u32, height as u32)
    }
}
