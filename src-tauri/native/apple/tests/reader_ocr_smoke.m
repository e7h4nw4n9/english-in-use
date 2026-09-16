#import <Foundation/Foundation.h>

extern char *reader_recognize_page(const char *path);
extern void reader_native_free(char *value);

// 原生烟雾检查：传入本地测试图片及可选的预期单词，不读写用户剪贴板。
int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc < 2) { fprintf(stderr, "需要指定测试图片路径\n"); return 2; }
        char *output = reader_recognize_page(argv[1]);
        if (!output) return 3;
        NSData *data = [[NSString stringWithUTF8String:output] dataUsingEncoding:NSUTF8StringEncoding];
        NSDictionary *result = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
        if (!result || result[@"error"]) puts(output);
        reader_native_free(output);
        if (!result || result[@"error"]) return 4;
        NSUInteger count = 0;
        BOOL found = argc < 3;
        for (NSDictionary *line in result[@"lines"]) {
            for (NSDictionary *word in line[@"words"]) {
                NSDictionary *rect = word[@"rect"];
                double x = [rect[@"x"] doubleValue], y = [rect[@"y"] doubleValue];
                double w = [rect[@"width"] doubleValue], h = [rect[@"height"] doubleValue];
                if (w <= 0 || h <= 0 || x < -.01 || y < -.01 || x + w > 1.01 || y + h > 1.01) return 5;
                if (argc > 2 && [word[@"text"] caseInsensitiveCompare:[NSString stringWithUTF8String:argv[2]]] == NSOrderedSame) found = YES;
                count++;
            }
        }
        printf("图片尺寸：%ld × %ld；识别行数：%lu；有效单词数：%lu；预期单词：%s\n",
            [result[@"width"] longValue], [result[@"height"] longValue],
            (unsigned long)[result[@"lines"] count], (unsigned long)count, found ? "已找到" : "未找到");
        return count > 0 && found ? 0 : 6;
    }
}
