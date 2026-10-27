#![doc = include_str!("../README.md")]
#[cfg_attr(feature = "off", doc = " Conditional function documentation.")]
pub fn conditional() {}
#[cfg_attr(any(), cfg_attr(all(), doc = " Nested conditional documentation."))]
pub struct Example {
    #[cfg_attr(any(), doc = " Conditional field documentation.")]
    pub value: bool,
}
impl Example {
    #[cfg_attr(feature = "off", doc = include_str!("../method.md",))]
    pub fn method(&self) {}
}
#[doc = include_str!("../missing.md")]
pub fn absent() {}
