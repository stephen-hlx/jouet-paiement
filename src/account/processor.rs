mod depositor;

pub(super) use depositor::Depositor;
pub(super) use depositor::DepositorError;

#[cfg(test)]
pub(super) use depositor::MockDepositor;
