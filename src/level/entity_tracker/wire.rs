use slotmap::new_key_type;

new_key_type! {
    pub struct WireKey;
}

pub type WireData = u32;

#[derive(Clone, Default, Debug)]
pub struct Wire {
    pub data: WireData,
}

impl Wire {
    /// # Panics
    ///
    /// Will panic if `channel` >= `WireData::BITS`
    pub fn set_channel(&mut self, channel: u8, value: bool) {
        assert!(channel < WireData::BITS as u8);
        let mask = 1 << channel as WireData;
        if value {
            self.data |= mask;
        } else {
            self.data &= !mask;
        }
    }
}
