//! Scalar functions exposed by the color worker, registered under `color.main`.

mod analysis;
mod convert;

use vgi::Worker;

/// Register every scalar function on the worker.
pub fn register(worker: &mut Worker) {
    worker.register_scalar(convert::ToHex);
    worker.register_scalar(convert::FromHex);
    worker.register_scalar(convert::RgbToHsl);
    worker.register_scalar(convert::HslToRgb);
    worker.register_scalar(convert::RgbToLab);
    worker.register_scalar(analysis::DeltaE);
    worker.register_scalar(analysis::Luminance);
    worker.register_scalar(analysis::ContrastRatio);
    worker.register_scalar(analysis::WcagLevel);
    worker.register_scalar(analysis::IsDark);
    worker.register_scalar(analysis::NearestColorName);
}
