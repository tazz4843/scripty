use deepspeech::Model;
use std::path::Path;
use std::sync::RwLock;

pub(crate) struct DsModel {
    pub(crate) model: RwLock<Model>
}

unsafe impl Send for DsModel {} // SAFETY: none, it just hasn't broken yet

unsafe impl Sync for DsModel {} // SAFETY: none, it just hasn't broken yet

impl DsModel {
    pub fn new(path: &Path) -> DsModel {
        DsModel {
            model: RwLock::new(Model::load_from_files(path).expect("failed to load model")),
        }
    }
}
