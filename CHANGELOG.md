# Changelog

## [v1.2.0](https://github.com/maddiemort/vexillologist/compare/v1.1.1...v1.2.0) (2024-10-06)

### Features

* add support for FoodGuessr score parsing and leaderboards
([a332fd9](https://github.com/maddiemort/vexillologist/commit/a332fd92c76f22db6847c55a94589cba6ed68f02))

## [v1.1.1](https://github.com/maddiemort/vexillologist/compare/v1.1.0...v1.1.1) (2024-10-06)

### Fixes

* order flagle scores descending when calculating best score
([42ea7c0](https://github.com/maddiemort/vexillologist/commit/42ea7c07b215e62ccb32f386108f75e4f2b6669d))
* stop markdown from interfering with Flagle score lists
([f6abd57](https://github.com/maddiemort/vexillologist/commit/f6abd57ed23a8316129aea4d65c95ac8c8518874))
* rank Flagle leaderboard entries the same if scores are the same
([794f7f9](https://github.com/maddiemort/vexillologist/commit/794f7f9c38f25179814b561c8059aa6d1067ed95))

## [v1.1.0](https://github.com/maddiemort/vexillologist/compare/v1.0.0...v1.1.0) (2024-10-06)

### Features

* add support for Flagle score parsing and leaderboards
([ca9c9ab](https://github.com/maddiemort/vexillologist/commit/ca9c9ab565fa5f1b7d711bf6cd7e6e56f6d07469))
* add "game" option for the `/leaderboard` command
([35b9637](https://github.com/maddiemort/vexillologist/commit/35b963722bce6562d364a64be3f3d882f71fecca))
* rename scores table to geogrid_scores
([f481376](https://github.com/maddiemort/vexillologist/commit/f48137660143f841a2228dde8a0d3d8b391dbbd4))

## v1.0.0 (2024-10-06)

### Features

* drop username column from users table
([f51a736](https://github.com/maddiemort/vexillologist/commit/f51a736fe9d0b138f04b013ebcc96866c1d2cb15))
* remove shuttle and run directly with serenity
([3e77f65](https://github.com/maddiemort/vexillologist/commit/3e77f65b46d7715ae656928ec644eaa23c87cbf0))
* rename weighted "score" to "medal points"
([08facd1](https://github.com/maddiemort/vexillologist/commit/08facd17eff9b6d0d09d22f52e9d4c68124b6e98))
* stop including user ID in medal sorting
([3ba0b63](https://github.com/maddiemort/vexillologist/commit/3ba0b63ab9d9b99ece2e6069558db310c1db6fcc))
* change medal sorting to use 4-2-1 weighted score
([ecb44e1](https://github.com/maddiemort/vexillologist/commit/ecb44e1d4b0140987130019f2f018277c01ac8c8))
* default late submissions to disabled in all-time GeoGrid leaderboard
([00f2c42](https://github.com/maddiemort/vexillologist/commit/00f2c426e527a75ee323a050dd40c6bfe3002bfa))
* allow turning off today's and late scores in all-time GeoGrid leaderboard
([fb0d6c2](https://github.com/maddiemort/vexillologist/commit/fb0d6c2bc4239b1c8759bc591b72b892e75e6512))
* log the guild ID in a couple of places
([a816650](https://github.com/maddiemort/vexillologist/commit/a81665016efc08552d3ca7c733721d8febd21678))
* include rerun footer in all-time GeoGrid leaderboard too
([0d57520](https://github.com/maddiemort/vexillologist/commit/0d5752029ba3f703ae555877ed425823551a7704))
* all-time GeoGrid leaderboard
([c351127](https://github.com/maddiemort/vexillologist/commit/c35112761d0e76b84e3e157857501bcad26f957c))
* daily GeoGrid leaderboard
([63e1304](https://github.com/maddiemort/vexillologist/commit/63e13041e2e6cc865f3ab33e9f400656d00c7ef0))
* take account of which day a GeoGrid score was submitted
([c11fde9](https://github.com/maddiemort/vexillologist/commit/c11fde90fa1f5a77aef7dfa52a1fe4e4260d49ce))
* basic GeoGrid score persistence
([5d33c1e](https://github.com/maddiemort/vexillologist/commit/5d33c1ece7033be6c8e3973aa592839016968dc9))
* switch to Shuttle, with serenity
([ee621a0](https://github.com/maddiemort/vexillologist/commit/ee621a08c60f649b61ed146594489e85dde3d58b))
* GeoGrid score parsing
([6fea808](https://github.com/maddiemort/vexillologist/commit/6fea808aa105c1bcdae915749dbe528fe630ba93))
* basic Discord bot from twilight example
([dc07667](https://github.com/maddiemort/vexillologist/commit/dc07667f7aaf1dbdc43f42c5748f741570618d36))
