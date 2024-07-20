use crate::common_skeleton::datafeed::event::MarketEvent;

// SlippageRegistry 结构体，包含市场事件的 Vec
#[derive(Debug, Default)]
pub struct SlippageRegistry<Data>{
    events: Vec<MarketEvent<Data>>,
}

impl <Data>SlippageRegistry<Data> {
    pub fn new() -> Self {
        SlippageRegistry {
            events: Vec::new(),
        }
    }

    pub fn register_event(&mut self, event: MarketEvent<Data>) {
        self.events.push(event);
    }
}

// 定义一个特征，包含 slippage_registry 字段
trait PendingWithSlippageRegistry<Data> {
    fn new() -> Self;
    fn register_slippage_event(&mut self, event: MarketEvent<Data>);
    fn get_slippage_registry(&self) -> &SlippageRegistry<Data>;
}

/// 为包含 [slippage_registry] 字段的 [PendingWithRegistry] 结构体实现特征
impl <Data>PendingWithSlippageRegistry<Data>for PendingWithRegistry<Data> {
    fn new() -> Self {
        PendingWithRegistry {
            slippage_registry: SlippageRegistry::new(),
        }
    }

    fn register_slippage_event(&mut self, event: MarketEvent<Data>) {
        self.slippage_registry.add_event(event);
    }

    fn get_slippage_registry(&self) -> &SlippageRegistry<Data> {
        &self.slippage_registry
    }
}

#[allow(dead_code)]
// 包含 slippage_registry 字段的结构体
pub struct PendingWithRegistry<Data> {
    slippage_registry: SlippageRegistry<Data>,
}