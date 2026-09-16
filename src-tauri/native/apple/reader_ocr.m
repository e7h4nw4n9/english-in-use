#import <Foundation/Foundation.h>
#import <Vision/Vision.h>
#import <ImageIO/ImageIO.h>
#import <TargetConditionals.h>
#if TARGET_OS_IPHONE
#import <UIKit/UIKit.h>
#else
#import <AppKit/AppKit.h>
#endif

// 返回独立拥有的 UTF-8 缓冲区，调用方必须使用 reader_native_free 释放。
static char *encode_result(NSDictionary *result) {
    NSData *data = [NSJSONSerialization dataWithJSONObject:result options:0 error:nil];
    if (!data) return strdup("{\"error\":\"无法编码识别结果\"}");
    char *output = malloc(data.length + 1);
    if (!output) return NULL;
    memcpy(output, data.bytes, data.length);
    output[data.length] = 0;
    return output;
}

// 将 Vision 左下角坐标转换为面向显示方向的左上角归一化坐标。
static NSDictionary *word_rect(CGRect rect) {
    return @{ @"x": @(rect.origin.x), @"y": @(1 - CGRectGetMaxY(rect)),
              @"width": @(rect.size.width), @"height": @(rect.size.height) };
}

// 在后台线程识别本地图片；路径由 Rust 资源解析器限定，不接受网页任意路径。
char *reader_recognize_page(const char *path) {
    @autoreleasepool {
        if (@available(macOS 10.15, iOS 13.0, *)) {
            NSURL *url = [NSURL fileURLWithPath:[NSString stringWithUTF8String:path]];
            CGImageSourceRef source = CGImageSourceCreateWithURL((__bridge CFURLRef)url, NULL);
            if (!source) return encode_result(@{@"error": @"无法读取书页图片"});
            NSDictionary *properties = CFBridgingRelease(CGImageSourceCopyPropertiesAtIndex(source, 0, NULL));
            CGImageRef image = CGImageSourceCreateImageAtIndex(source, 0, NULL);
            CFRelease(source);
            if (!image) return encode_result(@{@"error": @"无法解码书页图片"});
            NSInteger orientation = [properties[(__bridge NSString *)kCGImagePropertyOrientation] integerValue];
            if (orientation < 1 || orientation > 8) orientation = 1;
            NSUInteger width = CGImageGetWidth(image), height = CGImageGetHeight(image);
            if (orientation >= 5) { NSUInteger swap = width; width = height; height = swap; }
            VNRecognizeTextRequest *request = [[VNRecognizeTextRequest alloc] init];
            request.recognitionLevel = VNRequestTextRecognitionLevelAccurate;
            request.recognitionLanguages = @[@"en-US"];
            request.usesLanguageCorrection = YES;
            VNImageRequestHandler *handler = [[VNImageRequestHandler alloc] initWithCGImage:image
                orientation:(CGImagePropertyOrientation)orientation options:@{}];
            NSError *error = nil;
            BOOL success = [handler performRequests:@[request] error:&error];
            CGImageRelease(image);
            if (!success) return encode_result(@{@"error": error.localizedDescription ?: @"书页识别失败"});
            // 按识别行保留分词信息；本期不推断跨栏或跨页的阅读顺序。
            NSRegularExpression *words = [NSRegularExpression regularExpressionWithPattern:
                @"[\\p{Latin}\\p{M}]+(?:['’\\-][\\p{Latin}\\p{M}]+)*" options:0 error:nil];
            NSMutableArray *lines = [NSMutableArray array];
            for (VNRecognizedTextObservation *observation in request.results) {
                VNRecognizedText *candidate = [observation topCandidates:1].firstObject;
                if (!candidate) continue;
                NSMutableArray *tokens = [NSMutableArray array];
                for (NSTextCheckingResult *match in [words matchesInString:candidate.string options:0
                    range:NSMakeRange(0, candidate.string.length)]) {
                    VNRectangleObservation *box = [candidate boundingBoxForRange:match.range error:nil];
                    if (!box || CGRectIsEmpty(box.boundingBox)) continue;
                    [tokens addObject:@{@"text": [candidate.string substringWithRange:match.range],
                                        @"rect": word_rect(box.boundingBox)}];
                }
                // Vision 有时为斜杠两侧的不同词返回同一个框；无法区分时不猜选首词。
                NSMutableArray *distinctTokens = [NSMutableArray array];
                for (NSDictionary *token in tokens) {
                    NSUInteger sameBounds = 0;
                    for (NSDictionary *other in tokens) {
                        if ([token[@"rect"] isEqual:other[@"rect"]]) sameBounds++;
                    }
                    if (sameBounds == 1) [distinctTokens addObject:token];
                }
                [lines addObject:@{@"text": candidate.string, @"words": distinctTokens}];
            }
            return encode_result(@{@"width": @(width), @"height": @(height), @"lines": lines});
        }
        return encode_result(@{@"error": @"当前系统不支持原生文字识别"});
    }
}

// 只写剪贴板，不读取用户剪贴板；系统 UI 对象仅在主线程访问。
void reader_copy_word(const char *text) {
    NSString *word = [[NSString alloc] initWithUTF8String:text];
    void (^write)(void) = ^{
#if TARGET_OS_IPHONE
        UIPasteboard.generalPasteboard.string = word;
#else
        NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
        [pasteboard clearContents];
        [pasteboard setString:word forType:NSPasteboardTypeString];
#endif
    };
    if (NSThread.isMainThread) write();
    else dispatch_sync(dispatch_get_main_queue(), write);
}

// 释放跨越 C ABI 的结果内存。
void reader_native_free(char *value) { free(value); }
