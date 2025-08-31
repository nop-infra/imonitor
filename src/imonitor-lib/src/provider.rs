use crate::device::Device;
use idevice::provider::{IdeviceProvider, TcpProvider};

impl Device {
    pub fn get_provider(&self, label_suffix: &str) -> Box<dyn IdeviceProvider> {
        let mut provider: TcpProvider = self.into();
        provider.label.push('-');
        provider.label.push_str(label_suffix);
        Box::new(provider)
    }
}
