use super::*;

#[test]
fn frame_fullscreen_holds_main_vm() {
    let vm = FrameViewModel::FullScreen(ScreenViewModel::Main(MainViewModel {
        header: HeaderVM {
            profile: "default".into(),
            cache_hit_rate: "n/a".into(),
            configured: true,
        },
        messages: vec![MessageVM {
            text: "hello".into(),
        }],
        status: "Ready".into(),
        prompt: "> ".into(),
    }));
    match vm {
        FrameViewModel::FullScreen(ScreenViewModel::Main(_)) => {}
        _ => panic!("expected FullScreen(Main)"),
    }
}

#[test]
fn frame_modal_holds_background_and_popup() {
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
        title: "Select model".into(),
        items: vec![ListItemVM {
            label: "deepseek-chat".into(),
            hint: None,
            disabled: false,
            is_selected: true,
        }],
        selected: 0,
        scroll_offset: 0,
    });
    let vm = FrameViewModel::Modal {
        background: bg,
        popup,
    };
    match vm {
        FrameViewModel::Modal {
            background: _,
            popup: _,
        } => {}
        _ => panic!("expected Modal"),
    }
}

#[test]
fn step_view_model_constructs_with_input_field() {
    let vm = StepViewModel {
        title: "API KEY".into(),
        lines: vec![LineVM {
            text: "请输入 API KEY:".into(),
        }],
        input: Some(InputFieldVM {
            label: "api_key".into(),
            value: "sk-".into(),
            cursor_pos: 3,
        }),
        hints: vec![KeyHint {
            key: "Enter".into(),
            description: "下一步".into(),
        }],
        focused_field: 0,
    };
    assert_eq!(vm.title, "API KEY");
    assert!(vm.input.is_some());
    assert_eq!(vm.focused_field, 0);
}

#[test]
fn step_view_model_with_multiple_input_fields() {
    let vm = StepViewModel {
        title: "多字段".into(),
        lines: vec![],
        input: Some(InputFieldVM {
            label: "field1".into(),
            value: "v1".into(),
            cursor_pos: 2,
        }),
        hints: vec![],
        focused_field: 1,
    };
    assert_eq!(vm.focused_field, 1);
}

#[test]
fn main_view_model_empty_collections() {
    let vm = MainViewModel {
        header: HeaderVM {
            profile: "".into(),
            cache_hit_rate: "n/a".into(),
            configured: false,
        },
        messages: vec![],
        status: "".into(),
        prompt: "".into(),
    };
    assert!(!vm.header.configured);
    assert!(vm.messages.is_empty());
}
