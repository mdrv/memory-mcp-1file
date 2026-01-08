use super::Component;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ComponentRegistry {
    components: RwLock<Vec<Arc<dyn Component>>>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            components: RwLock::new(Vec::new()),
        }
    }

    pub async fn register(&self, component: Arc<dyn Component>) {
        let mut components = self.components.write().await;
        tracing::info!(name = component.name(), "Registering component");
        components.push(component);
    }

    pub async fn get_all(&self) -> Vec<Arc<dyn Component>> {
        self.components.read().await.clone()
    }

    pub async fn count(&self) -> usize {
        self.components.read().await.len()
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
