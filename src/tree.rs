pub trait Node: Sized {
    fn children_mut(&mut self) -> &mut Vec<Self>;
}

pub fn move_precondition(src_idx: &[usize], dst_idx: &[usize]) -> bool {
    !(dst_idx.len() >= src_idx.len() && src_idx == &dst_idx[..src_idx.len()])
}

pub fn move_<T: Node>(nodes: &mut Vec<T>, src_idx: &[usize], dst_idx: &[usize]) {
    if !move_precondition(src_idx, dst_idx) {
        return;
    }
    if let Some(node) = remove(nodes, src_idx) {
        insert(nodes, dst_idx, node);
    }
}

pub fn get_mut<'t, T: Node>(mut nodes: &'t mut [T], indices: &[usize]) -> Option<&'t mut T> {
    for i in 0..indices.len() {
        let idx = *indices.get(i)?;
        if i == indices.len() - 1 {
            return nodes.get_mut(idx);
        } else {
            nodes = nodes.get_mut(idx)?.children_mut();
        }
    }
    None
}

pub fn remove<T: Node>(mut nodes: &mut Vec<T>, indices: &[usize]) -> Option<T> {
    let mut index = None;
    for i in 0..indices.len() {
        let idx = indices[i];
        index = Some(idx);
        if i == indices.len() - 1 {
            break;
        } else {
            nodes = nodes.get_mut(idx)?.children_mut();
        }
    }
    index.map(|idx| nodes.remove(idx))
}

pub fn insert<T: Node>(mut nodes: &mut Vec<T>, indices: &[usize], node: T) {
    for &idx in indices {
        nodes = nodes[idx].children_mut();
    }
    nodes.push(node);
}
