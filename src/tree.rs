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
    // The tricky part is, if we remove the src index, it can invalidate the dst index at
    // the same level, so we need to update that index.
    let src_last_idx_idx = src_idx.len().saturating_sub(1);
    let mut dst_final_idx = dst_idx;
    let mut dst_new_idx_buf;
    if src_idx.get(src_last_idx_idx) < dst_idx.get(src_last_idx_idx) {
        dst_new_idx_buf = dst_idx.to_vec();
        dst_new_idx_buf[src_last_idx_idx] = dst_new_idx_buf[src_last_idx_idx].saturating_sub(1);
        dst_final_idx = &dst_new_idx_buf;
    }
    if let Some(node) = remove(nodes, src_idx) {
        insert(nodes, dst_final_idx, node);
    }
}

pub fn get_mut<'t, T: Node>(mut nodes: &'t mut [T], indices: &[usize]) -> Option<&'t mut T> {
    for i in 0..indices.len() {
        let idx = *indices.get(i)?;
        if i == indices.len() - 1 {
            return nodes.get_mut(idx);
        }
        nodes = nodes.get_mut(idx)?.children_mut();
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
        }
        nodes = nodes.get_mut(idx)?.children_mut();
    }
    index.map(|idx| nodes.remove(idx))
}

pub fn insert<T: Node>(mut nodes: &mut Vec<T>, indices: &[usize], node: T) {
    for &idx in indices {
        nodes = nodes[idx].children_mut();
    }
    nodes.push(node);
}

#[cfg(test)]
mod test {
    use super::move_;

    #[derive(PartialEq, Debug)]
    struct N(&'static str, Vec<Self>);
    impl super::Node for N {
        fn children_mut(&mut self) -> &mut Vec<Self> {
            &mut self.1
        }
    }
    #[test]
    fn test_move_b_to_a() {
        let mut nodes = vec![N("a", vec![N("a1", vec![])]), N("b", vec![N("b1", vec![])])];
        move_(&mut nodes, &[1], &[0]);
        assert_eq!(
            nodes,
            vec![N("a", vec![N("a1", vec![]), N("b", vec![N("b1", vec![])])]),]
        );
    }
    #[test]
    fn test_move_a_to_b() {
        let mut nodes = vec![N("a", vec![N("a1", vec![])]), N("b", vec![N("b1", vec![])])];
        move_(&mut nodes, &[0], &[1]);
        assert_eq!(
            nodes,
            vec![N("b", vec![N("b1", vec![]), N("a", vec![N("a1", vec![])])])]
        );
    }
}
