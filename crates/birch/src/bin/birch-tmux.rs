//! `birch-tmux` — the contrib adapter, shipped on `PATH`. The script in
//! `contrib/birch-tmux` is the source of truth; this only carries it.

#[path = "../adapter.rs"]
mod adapter;

fn main() {
    adapter::run(include_str!("../../../../contrib/birch-tmux"));
}
