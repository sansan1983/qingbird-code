use super::*;

fn welcome_vm() -> StepViewModel {
    StepViewModel {
        title: "欢迎".into(),
        lines: vec![LineVM {
            text: "欢迎使用 eflow".into(),
        }],
        input: None,
        hints: vec![KeyHint {
            key: "Enter".into(),
            description: "开始".into(),
        }],
        focused_field: 0,
    }
}

#[test]
fn engine_renders_step_to_draw_commands() {
    let engine = DefaultRenderEngine::new();
    let frame = FrameViewModel::FullScreen(ScreenViewModel::Wizard(welcome_vm()));
    let cmds = engine.render(&frame);
    assert!(!cmds.is_empty(), "engine must produce at least 1 command");
    // 第一个命令应该是 Block（带标题）
    match &cmds[0] {
        DrawCommand::Block { title, .. } => assert_eq!(title, "欢迎"),
        _ => panic!("expected first command to be Block with title"),
    }
}

#[test]
fn engine_renders_select_list_with_selected_prefix() {
    let engine = DefaultRenderEngine::new();
    let frame = FrameViewModel::FullScreen(ScreenViewModel::SelectList(SelectListViewModel {
        title: "选择语言".into(),
        items: vec![
            ListItemVM {
                label: "中文".into(),
                hint: None,
                disabled: false,
                is_selected: true,
            },
            ListItemVM {
                label: "English".into(),
                hint: None,
                disabled: false,
                is_selected: false,
            },
        ],
        selected: 0,
        scroll_offset: 0,
    }));
    let cmds = engine.render(&frame);
    // 必须包含 ▶ 前缀的 Span
    let has_selected = cmds
        .iter()
        .any(|c| matches!(c, DrawCommand::Span { prefix, .. } if prefix == "▶ "));
    assert!(
        has_selected,
        "engine must emit Span with ▶ prefix for selected"
    );
}

#[test]
fn engine_renders_modal_with_clear_area() {
    let engine = DefaultRenderEngine::new();
    let bg = ScreenViewModel::Main(MainViewModel {
        header: HeaderVM {
            profile: "p".into(),
            cache_hit_rate: "n/a".into(),
            configured: true,
        },
        messages: vec![],
        status: "Ready".into(),
        prompt: "".into(),
    });
    let popup = ScreenViewModel::SelectList(SelectListViewModel {
        title: "Select".into(),
        items: vec![],
        selected: 0,
        scroll_offset: 0,
    });
    let frame = FrameViewModel::Modal {
        background: bg,
        popup,
    };
    let cmds = engine.render(&frame);
    // 必须有 ClearArea（modal 弹出时清背景）
    let has_clear = cmds.iter().any(|c| matches!(c, DrawCommand::ClearArea));
    assert!(has_clear, "modal must emit ClearArea to dim background");
}
