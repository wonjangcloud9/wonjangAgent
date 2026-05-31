//! 공용 유틸리티.

use anyhow::{Context, Result};
use std::future::Future;

/// 비동기 작업을 동기 컨텍스트에서 실행한다.
///
/// 도구 트레이트(`Tool::execute`)는 동기이지만 일부 도구(웹, 서브에이전트)는
/// 비동기 작업이 필요하다. 현재 스레드가 이미 tokio 런타임 안에 있을 수 있어,
/// 전용 스레드에서 새 런타임으로 실행해 "중첩 런타임" 패닉을 피한다.
pub fn run_async<T, F>(fut: F) -> Result<T>
where
    F: Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || -> Result<T> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("런타임 생성 실패")?;
        rt.block_on(fut)
    })
    .join()
    .map_err(|_| anyhow::anyhow!("작업 스레드가 패닉했습니다"))?
}
