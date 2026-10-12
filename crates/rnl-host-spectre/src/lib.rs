//! placeholder until the Spectre adapter lands
use rnl_runtime::diag::Sink;
use rnl_runtime::host::HostDocument;
use rnl_runtime::source::Provider;
pub fn read(path: &str, provider: &dyn Provider, sink: &mut Sink) -> Option<HostDocument> { rnl_host_spice::read(path, provider, sink) }
