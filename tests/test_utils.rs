use zerg_pool::balancer::ZergRushSelector;
use zerg_pool::Process;
use tokio::time::Duration;

/// 测试专用的Selector构造器
pub struct TestSelectorBuilder {
    max_load: f64,
    warmup: Duration,
    backup_nodes: Vec<Process>,
}

impl TestSelectorBuilder {
    pub fn new(max_load: f64, warmup: Duration) -> Self {
        Self {
            max_load,
            warmup,
            backup_nodes: Vec::new(),
        }
    }

    pub fn with_backup_nodes(mut self, nodes: Vec<Process>) -> Self {
        self.backup_nodes = nodes;
        self
    }

    pub async fn build(self) -> ZergRushSelector {
        let mut selector = ZergRushSelector::new(self.max_load, self.warmup);
        for node in self.backup_nodes {
            selector.scale_in(&mut vec![node]).await.unwrap();
        }
        selector
    }
}
/// 测试专用的Process构造辅助函数
pub fn new_test_process(id: zerg_pool::ProcessId) -> zerg_pool::Process {
    zerg_pool::Process::new(
        id,
        vec![], // 空能力列表
        None    // 无最大任务限制
    )
}