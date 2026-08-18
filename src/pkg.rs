use serde::Deserialize;

use crate::src::{Location, Source, SourceId, Span};
use crate::syn::nodes::{Chunk, ChunkId};

#[derive(Debug)]
pub struct Package {
	manifest: Manifest,
	srcs: Vec<Source>,
	chunks: Vec<Chunk>,
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
	pub name: String,
	pub version: String,
}

	pub fn new() -> Self {
impl Package {
	pub fn new(manifest: Manifest) -> Self {
		Self {
			manifest,
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

	pub fn file(&self, id: ChunkId) -> &str {
		self.get_src(self.get_chunk(id).src).file()
	}

	pub fn chunk_ids(&self) -> impl ExactSizeIterator<Item = ChunkId> {
		(0..self.chunks.len() as u32).map(ChunkId::new)
	}

	pub fn add_chunk(&mut self, chunk: Chunk) -> ChunkId {
		let id = ChunkId::new(self.chunks.len() as u32);
		self.chunks.push(chunk);
		id
	}
}
