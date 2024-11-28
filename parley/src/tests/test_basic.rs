// Copyright 2024 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{testenv, Alignment, InlineBox};

#[test]
fn plain_multiline_text() {
    let mut env = testenv!();

    let text = "Hello world!\nLine 2\nLine 4";
    let mut builder = env.builder(text);
    let mut layout = builder.build(text);
    layout.break_all_lines(None);
    layout.align(None, Alignment::Start);

    env.check_snapshot(&layout);
}

#[test]
fn placing_inboxes() {
    let mut env = testenv!();

    for (position, test_case_name) in [
        (0, "start"),
        (3, "in_word"),
        (12, "end_nl"),
        (13, "start_nl"),
    ] {
        let text = "Hello world!\nLine 2\nLine 4";
        let mut builder = env.builder(text);
        builder.push_inline_box(InlineBox {
            id: 0,
            index: position,
            width: 10.0,
            height: 10.0,
        });
        let mut layout = builder.build(text);
        layout.break_all_lines(None);
        layout.align(None, Alignment::Start);
        env.check_snapshot_with_name(test_case_name, &layout);
    }
}