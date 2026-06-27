/// SSE 流式响应解析器（当前为 stub，实际实现留给后续版本）
pub struct SseStream;

impl SseStream {
    /// 解析 SSE 行，解析出 data 内容
    pub fn parse_line(line: &str) -> Option<&str> {
        line.strip_prefix("data: ")
    }

    /// 判断是否为流结束标记
    pub fn is_done(line: &str) -> bool {
        line.trim() == "data: [DONE]"
    }
}
