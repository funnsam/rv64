//! VirtIO devices

pub trait Device {
    const MAGIC: u32;
    const DEVICE_ID: u32;
    const VENDOR_ID: u32;
    const DEVICE_FEATURES: u32;
    const QUEUE_MAX: u32;

    fn get_driver_features(&self) -> u32;
    fn get_queue_pfn(&self) -> u32;
    fn get_status(&self) -> u32;

    fn set_driver_features(&self) -> u32;
    fn set_page_size(&self) -> u32;
    fn set_queue_sel(&self) -> u32;
    fn set_queue_num(&self) -> u32;
    fn set_queue_pfn(&self) -> u32;
    fn set_queue_notify(&self) -> u32;
    fn set_status(&self) -> u32;
}
