use crate::common_skeleton::datafeed::event::MarketEvent;

// SlippageRegistry 结构体，包含市场事件的 Vec
#[derive(Debug, Default)]
pub struct SlippageRegistry<Data>
{
    events: Vec<MarketEvent<Data>>,
}

impl<Data> SlippageRegistry<Data>
{
    pub fn new() -> Self
    {
        SlippageRegistry { events: Vec::new() }
    }

    pub fn register_event(&mut self, event: MarketEvent<Data>)
    {
        self.events.push(event);
    }
}

/// NOTE 在需要模拟延迟的回测场景下仅使用这种Pending状态。
#[allow(dead_code)]
pub struct SimulatedPending<Data>
{
    slippage_registry: SlippageRegistry<Data>,
}

#[allow(dead_code)]
trait PendingWithSlippageRegistry<Data>
{
    fn new() -> Self;
    fn register_slippage_event(&mut self, event: MarketEvent<Data>);
    fn get_slippage_registry(&self) -> &SlippageRegistry<Data>;
}

/// 为包含 [slippage_registry] 字段的 [SimulatedPending] 结构体实现特征
impl<Data> PendingWithSlippageRegistry<Data> for SimulatedPending<Data>
{
    fn new() -> Self
    {
        SimulatedPending { slippage_registry: SlippageRegistry::new() }
    }

    fn register_slippage_event(&mut self, event: MarketEvent<Data>)
    {
        self.slippage_registry.register_event(event);
    }

    fn get_slippage_registry(&self) -> &SlippageRegistry<Data>
    {
        &self.slippage_registry
    }
}
