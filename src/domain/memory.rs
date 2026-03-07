use crate::capability::memory::MemorySearchResult;

#[derive(Clone)]
pub struct Memory {
    pub content: String,
}

const MEMORY_DISTANCE_TIERS: [f64; 3] = [0.30, 0.38, 0.45];

impl From<MemorySearchResult> for Memory {
    fn from(value: MemorySearchResult) -> Self {
        Memory {
            content: value.content,
        }
    }
}

pub fn filter_memory_results(results: Vec<MemorySearchResult>) -> Vec<Memory> {
    for threshold in MEMORY_DISTANCE_TIERS {
        let filtered = results
            .iter()
            .filter(|memory| memory.distance <= threshold)
            .map(|memory| Memory {
                content: memory.content.clone(),
            })
            .collect::<Vec<Memory>>();
        if !filtered.is_empty() {
            return filtered;
        }
    }

    Vec::new()
}

impl Memory {
    pub fn to_list_text(memories: &Vec<Memory>) -> String {
        if memories.is_empty() {
            "None.".to_string()
        } else {
            memories
                .iter()
                .map(|memory| format!("- {}", memory.content))
                .collect::<Vec<String>>()
                .join("\n")
        }
    }
}
