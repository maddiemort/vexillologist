# Changelog

## [v1.5.0](https://github.com/maddiemort/vexillologist/compare/v1.4.0...74cbf5c13135d7a5bbbe17be94e0cc60ce979e87) (2025-06-28)

### Features

* list games you've participated in before but not yet today
([a22ecec](https://github.com/maddiemort/vexillologist/commit/a22ecec96d6afd2771d515620bad3412e816e51e))
* command to list all supported games
([865fd23](https://github.com/maddiemort/vexillologist/commit/865fd2384c92ddc73690bc9945ea44fe10afcfc8))
* make embed links clickable, link to the game's page
([f01cd9a](https://github.com/maddiemort/vexillologist/commit/f01cd9a7ad029a8f523cf94726efd1240e3e63aa))
* allow users to opt out of score tracking & leaderboards
([8e36195](https://github.com/maddiemort/vexillologist/commit/8e3619510376c1c806fd0ab4c729af4f453cbcae))
* new metric `vexillologist.messages_received_total`
([a147695](https://github.com/maddiemort/vexillologist/commit/a147695e14af31f388a6d1f2b2c0b72988410eb7))
* new metric `vexillologist.duplicate_scores_count`
([e7212de](https://github.com/maddiemort/vexillologist/commit/e7212dec426a33762047fdc1d37ea8762e3712f8))
* new metric `vexillologist.score_reactions_failed_count`
([5cfa715](https://github.com/maddiemort/vexillologist/commit/5cfa715a54389b9064e21ef7b559c42b2f1d1857))
* new metric `vexillologist.score_insertions_failed_count`
([17f377b](https://github.com/maddiemort/vexillologist/commit/17f377b4f6f15a729d19d20e1fd5934557517292))
* new metric `vexillologist.scores_inserted_count`
([a5e11d6](https://github.com/maddiemort/vexillologist/commit/a5e11d68467d18216cedaa12515a1dcdd822c98f))
* new metric `vexillologist.score_reactions_total`
([441d575](https://github.com/maddiemort/vexillologist/commit/441d5754dc565b328c2f9811599b6993ba4969a2))
* new metric `vexillologist.scores_received_total`
([1b5f684](https://github.com/maddiemort/vexillologist/commit/1b5f68434fdae34ea18bd01ebfb75384b964a0b1))
* support for Prometheus metrics scraping
([82a9286](https://github.com/maddiemort/vexillologist/commit/82a9286393150ec97787ce13c6bf651ec4e40961))
* support for direct Loki log exporting
([012c64c](https://github.com/maddiemort/vexillologist/commit/012c64c38cf636315e75ec98d8494a965514a4c3))

### Fixes

* export a value of 0 for all counter metrics at startup
([20247e9](https://github.com/maddiemort/vexillologist/commit/20247e94830700f2fe21168c32830f2cee49d4e4))

## [v1.4.0](https://github.com/maddiemort/vexillologist/compare/v1.3.0...v1.4.0) (2025-06-20)

### Features

* support for new Geogrid score format
([7fd9b2e](https://github.com/maddiemort/vexillologist/commit/7fd9b2e6a500c65aab32abe51d9417ed14ea3865))

## [v1.3.0](https://github.com/maddiemort/vexillologist/compare/v1.2.0...v1.3.0) (2025-02-15)

### Features

* leaderboard for a specific geogrid/flagle board number, not date
([afd9bc7](https://github.com/maddiemort/vexillologist/commit/afd9bc7c3059e20874d03931b01eb65d79a8af10))
* react to perfect scores with a crown emoji
([96be7c4](https://github.com/maddiemort/vexillologist/commit/96be7c47d2ddab451b70d5e302ebac8c57dced27))

### Fixes

* stop filtering out Flagle scores of 0 in leaderboards
([a482c63](https://github.com/maddiemort/vexillologist/commit/a482c638c581f7954d87d6893636a65e5eff81fe))

## [v1.2.0](https://github.com/maddiemort/vexillologist/compare/v1.1.1...v1.2.0) (2024-10-06)

### Features

* add support for FoodGuessr score parsing and leaderboards
([a332fd9](https://github.com/maddiemort/vexillologist/commit/a332fd92c76f22db6847c55a94589cba6ed68f02))

### [v1.1.1](https://github.com/maddiemort/vexillologist/compare/v1.1.0...v1.1.1) (2024-10-06)

#### Fixes

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
