use ::datamodel::Datamodel;
use ::builders::common::{Builder, Options};
use ::graph::CachedGraph;


pub struct LegacyBuilder<'a> {
    pub graph: &'a CachedGraph,
    pub options: &'a Options,
}


impl Options {
    pub fn legacy_defaults(datamodel: Datamodel) -> Options {
        Options {
            datamodel: datamodel,
            case_to_file_paths: Vec::new(),
            file_labels: Vec::new(),
            possible_associated_entites: Vec::new(),
            index_file_extensions: Vec::new(),
        }
    }
}


impl<'b> LegacyBuilder<'b> {
    pub fn new(options: &'b Options, graph: &'b CachedGraph) -> LegacyBuilder<'b> {
        LegacyBuilder { options: options, graph: graph }
    }
}


impl<'b> Builder for LegacyBuilder<'b> {
    fn options<'a>(&'a self) -> &'a Options {
        &self.options
    }

    fn graph<'a>(&'a self) -> &'a CachedGraph {
        &self.graph
    }
}
