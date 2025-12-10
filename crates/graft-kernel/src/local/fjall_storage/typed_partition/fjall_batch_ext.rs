use fjall::WriteBatch;

use super::TypedPartition;

use crate::local::fjall_storage::fjall_repr::FjallRepr;

pub trait FjallBatchExt {
    fn insert_typed<K: FjallRepr, V: FjallRepr>(
        &mut self,
        partition: &TypedPartition<K, V>,
        key: K,
        val: V,
    );

    fn remove_typed<K: FjallRepr, V: FjallRepr>(
        &mut self,
        partition: &TypedPartition<K, V>,
        key: K,
    );
}

impl FjallBatchExt for WriteBatch {
    fn insert_typed<K: FjallRepr, V: FjallRepr>(
        &mut self,
        partition: &TypedPartition<K, V>,
        key: K,
        val: V,
    ) {
        self.insert(&partition.keyspace, key.into_slice(), val.into_slice())
    }

    fn remove_typed<K: FjallRepr, V: FjallRepr>(
        &mut self,
        partition: &TypedPartition<K, V>,
        key: K,
    ) {
        self.remove(&partition.keyspace, key.into_slice())
    }
}
