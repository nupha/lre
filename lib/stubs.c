/*
 * Libregexp C stubs - 提供缺失的函数实现
 *
 * 这些函数在 libregexp 库中声明为需要用户提供的函数，
 * 但没有在生产环境中提供默认实现。
 */

#include <stdlib.h>
#include <stddef.h>

/* 检查栈溢出 - 在我们的实现中总是返回 0（不溢出） */
int lre_check_stack_overflow(void *opaque, size_t alloca_size) {
    (void)opaque;
    (void)alloca_size;
    return 0;
}

/* 检查超时 - 在我们的实现中总是返回 0（不超时） */
int lre_check_timeout(void *opaque) {
    (void)opaque;
    return 0;
}

/* 内存重分配 - 使用标准库的 realloc */
void *lre_realloc(void *opaque, void *ptr, size_t size) {
    (void)opaque;
    return realloc(ptr, size);
}