use std::sync::LazyLock;

use metrics::Unit;

pub static SCORES_RECEIVED: LazyLock<&'static str> = LazyLock::new(|| {
    let key = "vexillologist.scores_received_total";
    metrics::describe_counter!(
        key,
        Unit::Count,
        "Total number of messages received that contained a score"
    );
    key
});

pub static SCORES_INSERTED: LazyLock<&'static str> = LazyLock::new(|| {
    let key = "vexillologist.scores_inserted_count";
    metrics::describe_counter!(
        key,
        Unit::Count,
        "Number of received scores that were successfully inserted into the database"
    );
    key
});

pub static SCORE_INSERTIONS_FAILED: LazyLock<&'static str> = LazyLock::new(|| {
    let key = "vexillologist.score_insertions_failed_count";
    metrics::describe_counter!(
        key,
        Unit::Count,
        "Number of failed attempts to insert a received score into the database"
    );
    key
});

pub static SCORE_REACTIONS: LazyLock<&'static str> = LazyLock::new(|| {
    let key = "vexillologist.score_reactions_total";
    metrics::describe_counter!(
        key,
        Unit::Count,
        "Total number of attempts to add an emoji reaction to a message"
    );
    key
});
