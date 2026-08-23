pub(crate) const CEILING: u16 = u16::MAX;

pub(super) fn counted(shard: &str) -> String {
	format!(
		r"
select
	l.user_id::uinteger as user_id,
	l.recording_mbid::uuid as mbid,
	least(count(*), {CEILING})::usmallint as plays
from read_parquet({shard}) l
where l.recording_mbid is not null
group by 1, 2
"
	)
}
