#ifndef	_SETJMP_H
#define	_SETJMP_H

#ifdef __cplusplus
extern "C" {
#endif

typedef void* jmp_buf[256/sizeof(void *)];

#if __GNUC__ > 4 || (__GNUC__ == 4 && __GNUC_MINOR__ >= 1)
#define __returnstwice __attribute__((__returns_twice__))
#else
#define __returnstwice
#endif

#if __STDC_VERSION__ >= 201112L
#elif defined(__GNUC__)
#define _Noreturn __attribute__((__noreturn__))
#else
#define _Noreturn
#endif

typedef void* jmp_buf[256/sizeof(void *)];
__returnstwice int setjmp(jmp_buf buf);
_Noreturn int longjmp(jmp_buf buf, int arg);

#ifdef __cplusplus
}
#endif

#endif /* _SETJMP_H */
