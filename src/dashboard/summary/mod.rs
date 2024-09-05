pub mod data;
pub mod drawdown;
pub mod pnl;
pub mod trading;

use crate::common::account_positions::Position;
use prettytable::{Cell, Row, Table};

/// 该模块定义了一些用于处理交易数据和生成摘要表格的通用工具和接口。
///
/// # 原理介绍
/// 这个模块的主要目的是提供一套接口和工具，用于处理交易数据，并生成可视化的摘要表格。这对于金融交易系统非常有用，可以帮助用户快速总结和查看交易的关键数据。
///
/// 具体来说：
/// - `Initialiser` 特性（trait）定义了一个初始化器接口，它要求实现者能够通过配置初始化自身。
/// - `PositionSummariser` 特性定义了一个更新和生成交易仓位摘要的接口。通过实现这个接口，可以在处理一组交易仓位时快速生成统计数据。
/// - `TableBuilder` 特性定义了一套接口，用于生成表格标题、行数据以及完整表格。它还支持将多个表格合并为一个表格。
/// - `combine` 函数用于合并多个表格生成器，生成一个包含所有行数据的完整表格。
pub trait Initialiser
{
    type Config;
    fn init(config: Self::Config) -> Self;
}

/// 用于生成交易仓位摘要的接口，提供更新仓位和生成仓位摘要的功能。
pub trait PositionSummariser
{
    /// 更新当前仓位摘要，根据传入的 `Position` 对象更新内部状态。
    fn update(&mut self, position: &Position);

    /// 生成仓位摘要，根据一组 `Position` 对象迭代调用 `update` 方法。
    fn generate_summary(&mut self, positions: &[Position])
    {
        for position in positions.iter() {
            self.update(position)
        }
    }
}

/// 用于生成表格的接口，提供生成标题行、数据行和完整表格的功能。
pub trait TableBuilder
{
    /// 返回表格的标题行。
    fn titles(&self) -> Row;

    /// 返回表格的一行数据。
    fn row(&self) -> Row;

    /// 生成带有 ID 列的完整表格。
    ///
    /// # 参数
    /// - `id_cell`: 表格第一列的 ID 字符串
    ///
    /// # 返回
    /// 返回包含标题和一行数据的表格。
    fn table(&self, id_cell: &str) -> Table
    {
        let mut table = Table::new();

        let mut titles = self.titles();
        titles.insert_cell(0, Cell::new(""));
        table.set_titles(titles);

        let mut row = self.row();
        row.insert_cell(0, Cell::new(id_cell));
        table.add_row(row);

        table
    }

    /// 生成包含两个表格数据的完整表格，每个表格都有自己的 ID 列。
    ///
    /// # 参数
    /// - `id_cell`: 第一个表格的 ID 字符串
    /// - `another`: 另一个表格生成器及其对应的 ID 字符串
    ///
    /// # 返回
    /// 返回包含两个表格数据的完整表格。
    fn table_with<T: TableBuilder>(&self, id_cell: &str, another: (T, &str)) -> Table
    {
        let mut table = Table::new();

        let mut titles = self.titles();
        titles.insert_cell(0, Cell::new(""));
        table.set_titles(titles);

        let mut first_row = self.row();
        first_row.insert_cell(0, Cell::new(id_cell));
        table.add_row(first_row);

        let mut another_row = another.0.row();
        another_row.insert_cell(0, Cell::new(another.1));
        table.add_row(another_row);

        table
    }
}

/// 合并多个表格生成器，生成一个包含所有行数据的完整表格。
///
/// # 参数
/// - `builders`: 表格生成器的迭代器，每个生成器都包含一个 ID 字符串和实现了 `TableBuilder` 特性的对象。
///
/// # 返回
/// 返回包含所有行数据的合并表格。
pub fn combine<Iter, T>(builders: Iter) -> Table
    where Iter: IntoIterator<Item = (String, T)>,
          T: TableBuilder
{
    builders.into_iter().enumerate().fold(Table::new(), |mut table, (index, (id, builder))| {
                                        // 使用第一个生成器设置表格标题
                                        if index == 0 {
                                            let mut titles = builder.titles();
                                            titles.insert_cell(0, Cell::new(""));
                                            table.set_titles(titles);
                                        }

                                        // 为每个生成器添加行数据
                                        let mut row = builder.row();
                                        row.insert_cell(0, Cell::new(&id));
                                        table.add_row(row);

                                        table
                                    })
}
