use crate::src::{Location, Source, SourceId, Span};
use crate::syn::{Chunk, ChunkId};

#[derive(Debug)]
pub struct Package {
	srcs: Vec<Source>,
	chunks: Vec<Chunk>,
}

impl Package {
	pub fn new() -> Self {
		Self {
			srcs: Vec::new(),
			chunks: Vec::new(),
		}
	}

	pub fn get_src(&self, id: SourceId) -> &Source {
		&self.srcs[id.index()]
	}

	pub fn add_src(&mut self, file: String, text: String) -> SourceId {
		let id = SourceId::new(self.srcs.len() as u32);
		self.srcs.push(Source::new(id, file, text));
		id
	}

	pub fn loc(&self, span: Span) -> Location {
		self.get_src(span.src).loc(span.start)
	}

	pub fn get_chunk(&self, id: ChunkId) -> &Chunk {
		&self.chunks[id.index()]
	}

	pub fn chunk_ids(&self) -> impl Iterator<Item = ChunkId> {
		(0..self.chunks.len() as u32).map(ChunkId::new)
	}

	pub fn add_chunk(&mut self, chunk: Chunk) -> ChunkId {
		let id = ChunkId::new(self.chunks.len() as u32);
		self.chunks.push(chunk);
		id
	}
}
