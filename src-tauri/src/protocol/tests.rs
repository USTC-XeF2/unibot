#[cfg(test)]
mod tests {
    use crate::models::{InternalEvent, MessageSegment, MessageSource};
    use crate::protocol::adapter::{internal_to_milky_segment, milky_to_internal_segment};
    use crate::protocol::types::{
        ApiResponse, BotRuntimeContext, MilkyAdapter, MilkySegment, ProtocolAdapter,
    };

    #[test]
    fn text_segment_roundtrip() {
        let internal = MessageSegment::Text {
            text: "hello".to_string(),
        };
        let milky = internal_to_milky_segment(&internal);
        let back = milky_to_internal_segment(&milky);
        assert_eq!(internal, back);
    }

    #[test]
    fn image_segment_roundtrip() {
        let internal = MessageSegment::Image {
            file: "test.png".to_string(),
            url: "http://example.com/test.png".to_string(),
        };
        let milky = internal_to_milky_segment(&internal);
        let back = milky_to_internal_segment(&milky);
        assert_eq!(internal, back);
    }

    #[test]
    fn at_segment_qq_conversion() {
        let internal = MessageSegment::At {
            target: "12345".to_string(),
        };
        let milky = internal_to_milky_segment(&internal);
        match milky {
            MilkySegment::At { qq } => assert_eq!(qq, "12345"),
            _ => panic!("expected At segment"),
        }
    }

    #[test]
    fn at_all_segment_roundtrip() {
        let internal = MessageSegment::AtAll;
        let milky = internal_to_milky_segment(&internal);
        let back = milky_to_internal_segment(&milky);
        assert_eq!(internal, back);
    }

    #[test]
    fn face_segment_roundtrip() {
        let internal = MessageSegment::Face {
            id: "123".to_string(),
        };
        let milky = internal_to_milky_segment(&internal);
        let back = milky_to_internal_segment(&milky);
        assert_eq!(internal, back);
    }

    #[test]
    fn api_response_ok_serialization() {
        let ok = ApiResponse::ok(serde_json::json!({"user_id": 10001}));
        let json = serde_json::to_string(&ok).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"retcode\":0"));
        assert!(json.contains("\"user_id\":10001"));
    }

    #[test]
    fn api_response_failed_serialization() {
        let failed = ApiResponse::failed(-400, "bad request");
        let json = serde_json::to_string(&failed).unwrap();
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"retcode\":-400"));
        assert!(json.contains("\"message\":\"bad request\""));
    }

    #[test]
    fn api_response_not_found_serialization() {
        let failed = ApiResponse::failed(-404, "not found");
        let json = serde_json::to_string(&failed).unwrap();
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"retcode\":-404"));
    }

    #[test]
    fn adapter_message_event_conversion_private() {
        let adapter = MilkyAdapter::new();
        let event = InternalEvent::Message {
            message_id: "msg-1".to_string(),
            message_seq: 42,
            sender_user_id: "10001".to_string(),
            source: MessageSource::Private {
                peer_user_id: "10002".to_string(),
            },
            content: vec![MessageSegment::Text {
                text: "hello".to_string(),
            }],
            origin_bot_id: None,
            time: 1710000000000,
        };

        let milky_event = adapter.adapt_event(
            &event,
            &BotRuntimeContext {
                bot_id: "bot-1".to_string(),
                bound_user_id: "10001".to_string(),
                access_token: "token".to_string(),
                listen_addr: "127.0.0.1:3001".parse().unwrap(),
            },
        );

        assert!(milky_event.is_some());
        let milky_event = milky_event.unwrap();
        assert_eq!(milky_event.event_type, "message_receive");
        assert_eq!(milky_event.time, 1710000000); // ms to s
        assert_eq!(milky_event.self_id, "bot-1");
    }

    #[test]
    fn adapter_message_event_conversion_group() {
        let adapter = MilkyAdapter::new();
        let event = InternalEvent::Message {
            message_id: "msg-2".to_string(),
            message_seq: 1,
            sender_user_id: "10001".to_string(),
            source: MessageSource::Group {
                group_id: "123456".to_string(),
            },
            content: vec![MessageSegment::Text {
                text: "group hello".to_string(),
            }],
            origin_bot_id: None,
            time: 1710000001000,
        };

        let milky_event = adapter.adapt_event(
            &event,
            &BotRuntimeContext {
                bot_id: "bot-1".to_string(),
                bound_user_id: "10001".to_string(),
                access_token: "token".to_string(),
                listen_addr: "127.0.0.1:3001".parse().unwrap(),
            },
        );

        assert!(milky_event.is_some());
        let milky_event = milky_event.unwrap();
        assert_eq!(milky_event.event_type, "message_receive");
        assert_eq!(milky_event.time, 1710000001);
    }

    #[test]
    fn adapter_friend_request_event_conversion() {
        let adapter = MilkyAdapter::new();
        let event = InternalEvent::FriendRequestCreated {
            request_id: "req-1".to_string(),
            initiator_user_id: "10002".to_string(),
            target_user_id: "10001".to_string(),
            time: 1710000002000,
        };

        let milky_event = adapter.adapt_event(
            &event,
            &BotRuntimeContext {
                bot_id: "bot-1".to_string(),
                bound_user_id: "10001".to_string(),
                access_token: "token".to_string(),
                listen_addr: "127.0.0.1:3001".parse().unwrap(),
            },
        );

        assert!(milky_event.is_some());
        let milky_event = milky_event.unwrap();
        assert_eq!(milky_event.event_type, "friend_request");
        assert_eq!(milky_event.time, 1710000002);
        assert_eq!(milky_event.self_id, "bot-1");
    }

    #[test]
    fn adapter_group_member_increase_event_conversion() {
        let adapter = MilkyAdapter::new();
        let event = InternalEvent::GroupMemberJoined {
            group_id: "123456".to_string(),
            operator_user_id: "10003".to_string(),
            target_user_id: "10004".to_string(),
            time: 1710000003000,
        };

        let milky_event = adapter.adapt_event(
            &event,
            &BotRuntimeContext {
                bot_id: "bot-1".to_string(),
                bound_user_id: "10001".to_string(),
                access_token: "token".to_string(),
                listen_addr: "127.0.0.1:3001".parse().unwrap(),
            },
        );

        assert!(milky_event.is_some());
        let milky_event = milky_event.unwrap();
        assert_eq!(milky_event.event_type, "group_member_increase");
        assert_eq!(milky_event.time, 1710000003);
    }

    #[test]
    fn adapter_group_member_decrease_event_conversion() {
        let adapter = MilkyAdapter::new();
        let event = InternalEvent::GroupMemberLeft {
            group_id: "123456".to_string(),
            operator_user_id: Some("10003".to_string()),
            target_user_id: "10004".to_string(),
            time: 1710000004000,
        };

        let milky_event = adapter.adapt_event(
            &event,
            &BotRuntimeContext {
                bot_id: "bot-1".to_string(),
                bound_user_id: "10001".to_string(),
                access_token: "token".to_string(),
                listen_addr: "127.0.0.1:3001".parse().unwrap(),
            },
        );

        assert!(milky_event.is_some());
        let milky_event = milky_event.unwrap();
        assert_eq!(milky_event.event_type, "group_member_decrease");
        assert_eq!(milky_event.time, 1710000004);
    }

    #[test]
    fn adapter_login_info_conversion() {
        let adapter = MilkyAdapter::new();
        let user = crate::models::UserProfile {
            user_id: "10001".to_string(),
            nickname: "Alice".to_string(),
            avatar: "http://example.com/avatar.jpg".to_string(),
            signature: "Hello world".to_string(),
            account_status: Default::default(),
        };

        let login_info = adapter.adapt_login_info(&user);
        assert_eq!(login_info["user_id"], 10001);
        assert_eq!(login_info["nickname"], "Alice");
    }

    #[test]
    fn adapter_message_send_conversion() {
        let adapter = MilkyAdapter::new();
        let result = adapter.adapt_message_send("msg-123", 42);
        assert_eq!(result["message_id"], "msg-123");
        assert_eq!(result["message_seq"], 42);
    }

    #[test]
    fn adapter_error_conversion() {
        let adapter = MilkyAdapter::new();

        let validation = crate::error::AppError::Validation("bad input".to_string());
        let (code, msg) = adapter.adapt_error(&validation);
        assert_eq!(code, -400);
        assert_eq!(msg, "bad input");

        let not_found = crate::error::AppError::NotFound("missing".to_string());
        let (code, msg) = adapter.adapt_error(&not_found);
        assert_eq!(code, -404);
        assert_eq!(msg, "missing");

        let internal = crate::error::AppError::Internal("server error".to_string());
        let (code, msg) = adapter.adapt_error(&internal);
        assert_eq!(code, -500);
        assert_eq!(msg, "server error");
    }

    #[test]
    fn multiple_segments_conversion() {
        let internal = vec![
            MessageSegment::Text {
                text: "Hello ".to_string(),
            },
            MessageSegment::At {
                target: "12345".to_string(),
            },
            MessageSegment::Text {
                text: "!".to_string(),
            },
        ];
        let milky = crate::protocol::adapter::internal_to_milky_segments(&internal);
        assert_eq!(milky.len(), 3);
        assert!(matches!(milky[0], MilkySegment::Text { .. }));
        assert!(matches!(milky[1], MilkySegment::At { .. }));
        assert!(matches!(milky[2], MilkySegment::Text { .. }));

        let back = crate::protocol::adapter::milky_to_internal_segments(&milky);
        assert_eq!(internal, back);
    }
}
