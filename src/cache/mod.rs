extern crate time;

use std::collections::{HashMap};
use std::cmp::Ordering;
use time::*;
use dns::message::*;


///Unbounded cache of DnsAnswer
///It tries to be somewhat performant by using a HashMap for lookups and keeping
///an ordered Vec of keys by expiry for fast removal of expired items. 
pub struct Cache {
    map: HashMap<CacheKey,CacheEntry>, //for retrieval
    keys: Vec<CacheExpiry> //for expiring (ordered). BTreeSet/Map doesn't work because it does't have any way to iterate and remove
}

impl Default for Cache  {
    fn default() -> Cache {
        Cache {
            map: HashMap::new(),
            keys: Vec::new()
        }
    }
}

pub trait Expires {
    fn expiry(&self) -> SteadyTime;
}

impl Cache  {
    pub fn upsert(&mut self, key: CacheKey, val: CacheEntry) {
        self.remove_expired();
        let expiry_data = CacheExpiry::new(key.clone(), val.expiry());
        debug!("Cached answer with key {:?}", key);
        self.keys.insert(0, expiry_data);
        self.keys.sort(); //only 1 item should ever be out-of-order. 
        self.map.entry(key).or_insert(val);        
        debug!("There are {} keys and {} map entries", self.keys.len(), self.map.len());
    }

    pub fn get(&self, key: &CacheKey) -> Option<&CacheEntry> {
        self.map.get(key)
    }

    #[allow(dead_code)]
    pub fn contains(&self, key: &CacheKey) -> bool {
        self.map.contains_key(&key)
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn remove_expired(&mut self) -> usize {
        let now = SteadyTime::now();        
        let key_count = self.keys.len();

        self.remove_expired_map(now);
        self.keys.retain(|cache_expiry| cache_expiry.expiry > now);

        debug_assert!(self.map.len() == self.keys.len(), format!("map.len {:?} != keys.len {:?} map {:?} keys {:?}", self.map.len(), self.keys.len(), self.map, self.keys));        
        key_count - self.keys.len()
    }

    fn remove_expired_map(&mut self, now: SteadyTime) {
        //keys are ordered by expiry
        for cache_expiry in &self.keys {
            if cache_expiry.expiry > now {
                break;
            }
            debug!("Removing {:?} with expiry {:?}", &cache_expiry.key, cache_expiry.expiry);
            self.map.remove(&cache_expiry.key);
        }
    }
}

#[derive(Eq)]
#[derive(PartialEq)]
#[derive(PartialOrd)]
#[derive(Hash)]
#[derive(Clone)]
#[derive(Debug)]
pub struct CacheKey {
    qname: String,
    qtype: u16,
    qclass: u16
}

impl CacheKey {
    pub fn new(qname: String, qtype: u16, qclass: u16) -> CacheKey {
        CacheKey {
            qname: qname,
            qtype: qtype,
            qclass: qclass
        }
    }

    pub fn from(query: &DnsQuestion) -> CacheKey {
        CacheKey::new(query.qname.to_string(), query.qtype, query.qclass)        
    }
}

impl Ord for CacheKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.qname.cmp(&other.qname)
    }
}

#[derive(PartialOrd)]
#[derive(PartialEq)]
#[derive(Eq)]
#[derive(Debug)]
pub struct CacheEntry {
    pub key: CacheKey, //for expiring
    pub answers: Vec<DnsAnswer>,
    ttl: u32,
    expiry: SteadyTime
}

impl CacheEntry {
    pub fn new(key: CacheKey, answers: Vec<DnsAnswer>, ttl: u32) -> CacheEntry {
        CacheEntry {
            key: key,
            answers: answers,
            ttl: ttl,
            expiry: SteadyTime::now() + Duration::seconds(ttl as i64)
        }
    }

    pub fn from(msg: &DnsMessage) -> Option<CacheEntry> {        
        if let Some(answer) = msg.first_answer() {
            let a = answer.clone();
            let key = CacheKey::new(a.name.to_string(), a.atype, a.aclass);
            return Some(CacheEntry::new(key, msg.clone().answers, answer.ttl))
        } else {
            warn!("No answer in {:?}", msg);
        }
        None
    }

    pub fn calc_ttl(&self) -> u32 {
        let now = SteadyTime::now();
        if self.expiry > now {
            let ttl = (self.expiry - now).num_seconds() as u32;
            return ttl;
        }
        0
    }
}

impl Ord for CacheEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.expiry.cmp(&other.expiry)
    }
}

impl Expires for CacheEntry {
    fn expiry(&self) -> SteadyTime {
        self.expiry
    }
}

#[derive(PartialOrd)]
#[derive(PartialEq)]
#[derive(Eq)]
#[derive(Debug)]
pub struct CacheExpiry {
    pub key: CacheKey,
    pub expiry: SteadyTime
}

impl CacheExpiry {
    pub fn new(key: CacheKey, expiry: SteadyTime) -> CacheExpiry {
        CacheExpiry {
            key: key,
            expiry: expiry
        }
    }
}

impl Ord for CacheExpiry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.expiry.cmp(&other.expiry)
    }
}

#[cfg(test)]
mod test {
    use super::{Cache, CacheEntry, CacheKey};
    use std::thread;
    use std::time::Duration;
    use std::str::FromStr;
    use dns::message::{DnsAnswer, DnsName};

    fn test_cache() -> Cache {
        let mut cache = Cache::default();
        let key = CacheKey::new(String::from("yahoo.com"), 1, 1);
        let val = CacheEntry::new(key.clone(), test_answers(), 5);
        cache.upsert(key.clone(), val);
        cache
    }

    fn test_key_with(name: String) -> CacheKey {
        CacheKey::new(name, 1, 1)
    }

    fn test_key() -> CacheKey {
        test_key_with(String::from("yahoo.com"))
    }

    fn test_answers() -> Vec<DnsAnswer> {
        vec![test_answer()]
    }

    fn test_answers_with(domain: String) -> Vec<DnsAnswer> {
        vec![test_answer_with(domain)]
    }

    fn test_answer() -> DnsAnswer {
        test_answer_with(String::from("yahoo.com"))
    }

    fn test_answer_with(domain: String) -> DnsAnswer {
        DnsAnswer::new(DnsName::from_string(domain), 1, 1, 10, 4, vec![200, 200, 200, 200])
    }

    #[test]
    fn upsert() {
        let cache = test_cache();
        let key = test_key();
        assert_eq!(cache.get(&key).unwrap().answers[0].name, DnsName::from_str("yahoo.com").unwrap());
    }

    #[test]
    fn expiry() {
        let mut cache = test_cache();
        let key2 = CacheKey::new(String::from("lycos.com"), 1, 1);
        let val2 = CacheEntry::new(key2.clone(), test_answers_with(String::from("lycos.com")), 1);
        cache.upsert(key2, val2);

        assert_eq!(2, cache.len());
        thread::sleep(Duration::from_millis(1010));
        assert_eq!(1, cache.remove_expired());
        assert_eq!(1, cache.len());
    }

     #[test]
    fn len() {
        let cache = test_cache();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn contains() {
        let cache = test_cache();
        let key = CacheKey::new(String::from("yahoo.com"), 1, 1);
        assert!(cache.contains(&key));
    }
}
