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

pub static SCORE_REACTIONS: LazyLock<&'static str> = LazyLock::new(|| {
    let key = "vexillologist.score_reactions_total";
    metrics::describe_counter!(
        key,
        Unit::Count,
        "Total number of attempts to add an emoji reaction to a message"
    );
    key
});
