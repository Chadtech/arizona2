use crate::capability::memory::MemorySearchResult;

pub struct Memory {
    pub content: String,
}

impl From<MemorySearchResult> for Memory {
    fn from(value: MemorySearchResult) -> Self {
        Memory {
            content: value.content,
        }
    }
}
