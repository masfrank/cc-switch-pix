use crate::database::Database;
use crate::services::{ProxyService, PxpipeService, UsageCache};
use std::sync::Arc;

/// 全局应用状态
pub struct AppState {
    pub db: Arc<Database>,
    pub proxy_service: ProxyService,
    pub pxpipe_service: PxpipeService,
    pub usage_cache: Arc<UsageCache>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(db: Arc<Database>) -> Self {
        let proxy_service = ProxyService::new(db.clone());
        let pxpipe_service = PxpipeService::new(db.clone());

        Self {
            db,
            proxy_service,
            pxpipe_service,
            usage_cache: Arc::new(UsageCache::new()),
        }
    }
}
