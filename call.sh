#!/usr/bin/env bash

USER_AGENT="Alexdelia's personal declarative listen/0.1.0 ( https://github.com/Alexdelia/listen )"

RECORD="fb5a54ff-5210-4dd6-ab1d-601a94089a4d"

curl \
	-A "$USER_AGENT" \
	-H "content-type: application/json" \
	"https://musicbrainz.org/ws/2/recording/${RECORD}?fmt=json\
&inc=releases+url-rels"

RELEASE="273dbb2e-4306-4844-a8d5-e1dda14685ff"

# curl \
# 	-A "$USER_AGENT" \
# 	-H "content-type: application/json" \
# 	"https://musicbrainz.org/ws/2/release/${RELEASE}?fmt=json\
# &inc=url-rels"