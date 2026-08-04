use chrono::NaiveDate;

pub(super) const CREATED_FOR: &str = r#"{
	"count": 6,
	"offset": 0,
	"playlist_count": 6,
	"playlists": [
		{"playlist": {
			"date": "2026-06-30T11:00:00.000000+00:00",
			"extension": {"https://musicbrainz.org/doc/jspf#playlist": {
				"additional_metadata": {"algorithm_metadata": {"source_patch": "weekly-exploration"}}
			}},
			"identifier": "https://listenbrainz.org/playlist/11111111-1111-1111-1111-111111111111",
			"title": "Weekly Exploration for rob, week of 2026-06-30 Tue",
			"track": []
		}},
		{"playlist": {
			"date": "2026-08-03T22:01:18.076676+00:00",
			"extension": {"https://musicbrainz.org/doc/jspf#playlist": {
				"additional_metadata": {"algorithm_metadata": {"source_patch": "daily-jams"}}
			}},
			"identifier": "https://listenbrainz.org/playlist/22222222-2222-2222-2222-222222222222",
			"title": "Daily Jams for rob, 2026-08-04 Tue",
			"track": []
		}},
		{"playlist": {
			"date": "2026-07-12T12:24:12.184152+00:00",
			"extension": {"https://musicbrainz.org/doc/jspf#playlist": {
				"additional_metadata": {"algorithm_metadata": {"source_patch": "weekly-exploration"}}
			}},
			"identifier": "https://listenbrainz.org/playlist/33333333-3333-3333-3333-333333333333",
			"title": "Weekly Exploration for rob, week of 2026-07-12 Tue",
			"track": []
		}},
		{"playlist": {
			"date": "2026-07-28T11:22:20.299249+00:00",
			"extension": {"https://musicbrainz.org/doc/jspf#playlist": {
				"additional_metadata": {"algorithm_metadata": {"source_patch": "weekly-jams"}}
			}},
			"identifier": "https://listenbrainz.org/playlist/44444444-4444-4444-4444-444444444444",
			"title": "Weekly Jams for rob, week of 2026-07-28 Tue",
			"track": []
		}},
		{"playlist": {
			"date": "2024-05-15T18:00:00.000000+00:00",
			"extension": {"https://musicbrainz.org/doc/jspf#playlist": {
				"collaborators": ["rob"],
				"public": true
			}},
			"identifier": "https://listenbrainz.org/playlist/55555555-5555-5555-5555-555555555555",
			"title": "lb-radio",
			"track": []
		}},
		{"playlist": {
			"date": "2026-07-28T12:24:12.184152+00:00",
			"extension": {"https://musicbrainz.org/doc/jspf#playlist": {
				"additional_metadata": {"algorithm_metadata": {"source_patch": "weekly-exploration"}}
			}},
			"identifier": "https://listenbrainz.org/playlist/66666666-6666-6666-6666-666666666666",
			"title": "Weekly Exploration for rob, week of 2026-07-28 Tue",
			"track": []
		}}
	]
}"#;

pub(super) const BARE_EXTENSION: &str = r#"{
	"playlists": [
		{"playlist": {
			"date": "2024-05-15T18:00:00.000000+00:00",
			"extension": {},
			"identifier": "https://listenbrainz.org/playlist/55555555-5555-5555-5555-555555555555",
			"title": "lb-radio",
			"track": []
		}},
		{"playlist": {
			"date": "2026-07-28T12:24:12.184152+00:00",
			"extension": {"https://musicbrainz.org/doc/jspf#playlist": {
				"additional_metadata": {"algorithm_metadata": {"source_patch": "weekly-exploration"}}
			}},
			"identifier": "https://listenbrainz.org/playlist/66666666-6666-6666-6666-666666666666",
			"title": "Weekly Exploration for rob, week of 2026-07-28 Tue",
			"track": []
		}}
	]
}"#;

pub(super) const PLAYLIST: &str = r#"{
	"playlist": {
		"annotation": "<p>The ListenBrainz Weekly Exploration playlist helps you discover new music!</p>",
		"creator": "listenbrainz",
		"date": "2026-07-12T12:24:12.184152+00:00",
		"extension": {"https://musicbrainz.org/doc/jspf#playlist": {
			"additional_metadata": {"algorithm_metadata": {"source_patch": "weekly-exploration"}},
			"created_for": "rob",
			"public": true
		}},
		"identifier": "https://listenbrainz.org/playlist/33333333-3333-3333-3333-333333333333",
		"title": "Weekly Exploration for rob, week of 2026-07-12 Tue",
		"track": [
			{
				"album": "Endless Summer",
				"creator": "The Midnight",
				"duration": 317228,
				"identifier": ["https://musicbrainz.org/recording/5ecaf4e8-c19d-4756-b697-20b8478b0c8c"],
				"title": "Vampires"
			},
			{
				"creator": "no recording",
				"identifier": ["https://musicbrainz.org/artist/46eb0fb7-9725-43af-97d7-6c717682a799"],
				"title": "artist only"
			},
			{
				"creator": "The Midnight",
				"identifier": ["https://musicbrainz.org/recording/aaaaaaaa-c19d-4756-b697-20b8478b0c8c"],
				"title": "Lost Boy"
			}
		]
	}
}"#;

pub(super) fn date(year: i32, month: u32, day: u32) -> NaiveDate {
	NaiveDate::from_ymd_opt(year, month, day).unwrap_or_default()
}
