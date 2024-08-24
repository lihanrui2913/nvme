#![no_std]

#[derive(Debug, Clone, Copy)]
pub struct NvmeNamespace {
    pub id: u32,
    pub blocks: u64,
    pub block_size: u64,
}

#[derive(Debug, Clone, Default)]
pub struct NvmeStats {
    pub completions: u64,
    pub submissions: u64,
}

extern crate alloc;

pub mod cmd;
pub mod memory;
pub mod nvme;
pub mod queues;
