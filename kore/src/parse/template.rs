use liquid_core::{Filter, ValueView, Runtime, Value};
use liquid_derive::{Display_filter, FilterReflection, ParseFilter};

#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "abs",
    description = "Returns the absolute value of a number.",
    parsed(AbsFilter) // A struct that implements `Filter` (must implement `Default`)
)]
pub struct Preview;

#[derive(Debug, Default, Display_filter)]
#[name = "preview"]
pub struct PreviewFilter;

impl Filter for PreviewFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &Runtime) -> Result<Value> {
    }
}
