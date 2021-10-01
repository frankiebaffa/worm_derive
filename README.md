# wORM Derive

A procedural macro add-on to [wORM](https://github.com/frankiebaffa/worm).

_ _ _

## Implementation

The following is how to implement **wORM**'s traits using **wORM Derive**.

Assume we have a database table named `AlbumTrackArtists` in a database file named `DecibelDb.db` whose creation script look something like this:

```sql
create table DecibelDb.AlbumTrackArtists
	(
		Id integer not null primary key autoincrement
	,	Artist_Id integer not null
	,	AlbumTrack_Id integer not null
	,	ArtistType_Id integer not null
	,	Active integer not null default 1
	,	CreatedDate text not null default current_timestamp
	,	LastEditDate text not null default current_timestamp
	,	foreign key (Artist_Id) references Artists (Id)
	,	foreign key (AlbumTrack_Id) references AlbumTracks (Id)
	,	foreign key (ArtistType_Id) references ArtistTypes (Id)
	);
create unique index DecibelDb.AlbumTrackArtistsUnique on AlbumTrackArtists
	(
		Artist_Id
	,	AlbumTrack_Id
	,	ArtistType_Id
	)
where Active = 1;
```

Rust's *struct* equivalent with this library's procedural macros would look like this:

```rust
use chrono::{DateTime, Local};
use worm_derive::Worm;
use crate::{artist::Artist, albumtrack::AlbumTrack, artisttype::ArtistType};
#[derive(Worm)]
#[dbmodel(table(db="DecibelDb", name="AlbumTrackArtists", alias="albumtrackartist"))]
pub struct AlbumTrackArtist {
    #[dbcolumn(column(name="Id", primary_key))]
    id: i64,
    #[dbcolumn(column(name="Artist_Id", foreign_key="Artist"))]
    artist_id: i64,
    #[dbcolumn(column(name="AlbumTrack_Id", foreign_key="AlbumTrack"))]
    albumtrack_id: i64,
    #[dbcolumn(column(name="ArtistType_Id", foreign_key="ArtistType"))]
    artisttype_id: i64,
    #[dbcolumn(column(name="Active", active_flag))]
    active: bool,
    #[dbcolumn(column(name="CreatedDate"))]
    createddate: DateTime<Local>,
    #[dbcolumn(column(name="LastEditDate"))]
    lasteditdate: DateTime<Local>,
}
```

The `primary_key` field on `id` implements the **PrimaryKey** trait from the **wORM** library. The same goes for the `foreign_key`s and `active_flag`.
