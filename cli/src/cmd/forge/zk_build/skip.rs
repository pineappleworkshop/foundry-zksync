/// A filter that excludes matching contracts from the build
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SkipBuildFilter {
    /// Exclude all `.t.sol` contracts
    Tests,
    /// Exclude all `.s.sol` contracts
    Scripts,
    /// Exclude if the file matches
    Custom(String),
}

impl SkipBuildFilter {
    /// Returns the pattern to match against a file
    pub fn file_pattern(&self) -> &str {
        match self {
            SkipBuildFilter::Tests => ".t.sol",
            SkipBuildFilter::Scripts => ".s.sol",
            SkipBuildFilter::Custom(s) => s.as_str(),
        }
    }
}

impl<T: AsRef<str>> From<T> for SkipBuildFilter {
    fn from(s: T) -> Self {
        match s.as_ref() {
            "test" | "tests" => SkipBuildFilter::Tests,
            "script" | "scripts" => SkipBuildFilter::Scripts,
            s => SkipBuildFilter::Custom(s.to_string()),
        }
    }
}
