use crate::common::account_positions::Position;

pub trait PositionSummariser: Copy
{
    fn update(&mut self, position: &Position);
    fn generate_summary(&mut self, positions: &[Position])
    {
        for position in positions.iter() {
            self.update(position)
        }
    }
}
