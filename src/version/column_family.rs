use crate::common::InternalKeyComparator;
use crate::memtable::Memtable;
use crate::options::ColumnFamilyOptions;
use crate::table::{TableCache, TableReader};
use crate::util::LRUCache;
use crate::version::{SuperVersion, Version};
use crate::ImmutableDBOptions;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct ColumnFamily {
    mem: Arc<Memtable>,
    imms: Vec<Arc<Memtable>>,
    super_version: Arc<SuperVersion>,

    // An ordinal representing the current SuperVersion. Updated by
    // InstallSuperVersion(), i.e. incremented every time super_version_
    // changes.
    super_version_number: Arc<AtomicU64>,
    version: Arc<Version>,
    cache: Arc<TableCache>,
    comparator: InternalKeyComparator,
    id: u32,

    name: String,
    options: Arc<ColumnFamilyOptions>,

    // The minimal log file which keep data of the memtable of this column family.
    // So the log files whose number is less than this value could be removed safely.
    log_number: u64,
}

impl ColumnFamily {
    pub fn new(
        id: u32,
        name: String,
        m: Memtable,
        comparator: InternalKeyComparator,
        version: Arc<Version>,
        options: ColumnFamilyOptions,
        db_options: Arc<ImmutableDBOptions>,
        cache: Arc<LRUCache<Box<dyn TableReader>>>,
    ) -> Self {
        let mem = Arc::new(m);
        let options = Arc::new(options);
        let cache = Arc::new(TableCache::new(cache, db_options, options.clone()));
        Self {
            log_number: version.get_log_number(),
            mem: mem.clone(),
            imms: Default::default(),
            super_version: Arc::new(SuperVersion {
                id,
                mem,
                imms: Default::default(),
                current: version.clone(),
                version_number: 0,
                column_family_options: options.clone(),
                cache: cache.clone(),
            }),
            super_version_number: Arc::new(AtomicU64::new(0)),
            version,
            id,
            comparator,
            name,
            options,
            cache,
        }
    }

    pub fn get_memtable(&self) -> Arc<Memtable> {
        self.mem.clone()
    }

    pub fn get_version(&self) -> Arc<Version> {
        self.version.clone()
    }

    pub fn get_options(&self) -> Arc<ColumnFamilyOptions> {
        self.options.clone()
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_log_number(&self) -> u64 {
        self.log_number
    }

    pub fn set_log_number(&mut self, log_number: u64) {
        self.log_number = log_number;
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn should_flush(&self) -> bool {
        false
    }

    pub fn get_super_version(&self) -> Arc<SuperVersion> {
        self.super_version.clone()
    }

    pub fn install_version(&mut self, next_log_number: u64, new_version: Version) -> Arc<Version> {
        self.remove(next_log_number);
        let super_version_number = self.super_version_number.fetch_add(1, Ordering::SeqCst) + 1;
        let version = Arc::new(new_version);
        let cache = self.cache.clone();
        let super_version = Arc::new(SuperVersion::new(
            self.id,
            self.mem.clone(),
            self.imms.clone(),
            version.clone(),
            self.options.clone(),
            cache,
            super_version_number,
        ));
        self.super_version = super_version;
        self.version = version.clone();
        if version.get_log_number() > 0 {
            self.log_number = version.get_log_number();
        }
        version
    }

    pub fn switch_memtable(&mut self, mem: Arc<Memtable>) {
        self.imms.push(self.mem.clone());
        let super_version_number = self.super_version_number.fetch_add(1, Ordering::SeqCst) + 1;
        let cache = self.cache.clone();
        let super_version = Arc::new(SuperVersion::new(
            self.id,
            mem.clone(),
            self.imms.clone(),
            self.version.clone(),
            self.options.clone(),
            cache,
            super_version_number,
        ));
        self.super_version = super_version;
        self.mem = mem;
    }

    pub fn create_memtable(&self, cf_id: u32, earliest_seq: u64) -> Memtable {
        Memtable::new(
            cf_id,
            self.options.write_buffer_size,
            self.comparator.clone(),
            earliest_seq,
        )
    }

    fn remove(&mut self, next_log_number: u64) {
        let mut imms = vec![];
        for m in &self.imms {
            if next_log_number > 0 && m.get_next_log_number() > next_log_number {
                imms.push(m.clone());
            }
        }
        self.imms = imms;
    }
}
