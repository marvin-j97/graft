use std::{
    borrow::Borrow,
    marker::PhantomData,
    ops::{Bound, RangeBounds},
};

use culprit::{Culprit, ResultExt};
use fjall::{Database, KeyspaceCreateOptions, Slice};
use tryiter::TryIteratorExt;

use crate::local::fjall_storage::{
    FjallStorageErr,
    fjall_repr::{FjallRepr, FjallReprRef},
    keys::FjallKeyPrefix,
};

pub mod fjall_batch_ext;

type Result<T> = culprit::Result<T, FjallStorageErr>;

#[derive(Clone)]
pub struct TypedPartition<K, V> {
    pub(super) keyspace: fjall::Keyspace,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V> TypedPartition<K, V>
where
    K: FjallRepr,
    V: FjallRepr,
{
    pub fn open(db: &Database, name: &str, opts: KeyspaceCreateOptions) -> Result<Self> {
        Ok(Self {
            keyspace: db.keyspace(name, opts)?,
            _phantom: PhantomData,
        })
    }

    #[inline]
    pub fn insert(&self, key: K, val: V) -> Result<()> {
        self.keyspace.insert(key.into_slice(), val.into_slice())?;
        Ok(())
    }

    #[inline]
    pub fn remove(&self, key: K) -> Result<()> {
        self.keyspace.remove(key.into_slice())?;
        Ok(())
    }

    #[inline]
    pub fn snapshot<'a>(&self, snapshot: &'a fjall::Snapshot) -> TypedPartitionSnapshot<'a, K, V> {
        TypedPartitionSnapshot {
            keyspace: self.keyspace.clone(),
            snapshot,
            _phantom: PhantomData,
        }
    }
}

pub struct TypedPartitionSnapshot<'a, K, V> {
    keyspace: fjall::Keyspace,
    snapshot: &'a fjall::Snapshot,
    _phantom: PhantomData<(K, V)>,
}

impl<'a, K, V> TypedPartitionSnapshot<'a, K, V>
where
    K: FjallRepr,
    V: FjallRepr,
{
    /// Returns `true` if this snapshot contains the provided key
    pub fn contains<B>(&self, key: &B) -> Result<bool>
    where
        B: FjallReprRef + ?Sized,
        K: Borrow<B>,
    {
        self.snapshot
            .contains_key(&self.keyspace, key.as_slice())
            .or_into_ctx()
    }

    /// Retrieve the value corresponding to the key
    pub fn get<B>(&self, key: &B) -> Result<Option<V>>
    where
        B: FjallReprRef + ?Sized,
        K: Borrow<B>,
    {
        if let Some(slice) = self.snapshot.get(&self.keyspace, key.as_slice())? {
            return Ok(Some(V::try_from_slice(slice).or_into_ctx()?));
        }
        Ok(None)
    }

    /// An optimized version of get when key is owned
    pub fn get_owned(&self, key: K) -> Result<Option<V>> {
        if let Some(slice) = self.snapshot.get(&self.keyspace, key.into_slice())? {
            return Ok(Some(V::try_from_slice(slice).or_into_ctx()?));
        }
        Ok(None)
    }

    pub fn range_keys<R: RangeBounds<K>>(
        &self,
        range: R,
    ) -> impl Iterator<Item = Result<K>> + use<R, K, V> {
        let r: (Bound<Slice>, Bound<Slice>) = (
            range.start_bound().map(|b| b.clone().into_slice()),
            range.end_bound().map(|b| b.clone().into_slice()),
        );
        self.snapshot
            .range(&self.keyspace, r)
            .err_into::<Culprit<FjallStorageErr>>()
            .map_ok(|(k, _)| K::try_from_slice(k).or_into_ctx())
    }

    pub fn range<R: RangeBounds<K>>(
        &self,
        range: R,
    ) -> impl Iterator<Item = Result<(K, V)>> + use<R, K, V> {
        let r: (Bound<Slice>, Bound<Slice>) = (
            range.start_bound().map(|b| b.clone().into_slice()),
            range.end_bound().map(|b| b.clone().into_slice()),
        );
        self.snapshot
            .range(&self.keyspace, r)
            .err_into::<Culprit<FjallStorageErr>>()
            .map_ok(|(k, v)| {
                Ok((
                    K::try_from_slice(k).or_into_ctx()?,
                    V::try_from_slice(v).or_into_ctx()?,
                ))
            })
    }

    /// iterate all of the values in the partition
    pub fn values(&self) -> impl Iterator<Item = Result<V>> + use<K, V> {
        self.snapshot
            .values(&self.keyspace)
            .err_into::<Culprit<FjallStorageErr>>()
            .map_ok(|v| V::try_from_slice(v).or_into_ctx())
    }

    pub fn prefix<'p, P>(
        &self,
        prefix: &'p P,
    ) -> impl Iterator<Item = Result<(K, V)>> + use<'p, P, K, V>
    where
        K: FjallKeyPrefix<Prefix = P>,
        P: AsRef<[u8]>,
    {
        self.snapshot
            .prefix(&self.keyspace, prefix)
            .err_into::<Culprit<FjallStorageErr>>()
            .map_ok(|(k, v)| {
                Ok((
                    K::try_from_slice(k).or_into_ctx()?,
                    V::try_from_slice(v).or_into_ctx()?,
                ))
            })
    }

    pub fn first<P>(&self, prefix: &P) -> Result<Option<(K, V)>>
    where
        K: FjallKeyPrefix<Prefix = P>,
        P: AsRef<[u8]>,
    {
        self.prefix(prefix).try_next()
    }
}
