/*
 * Loads truchet_ms.dll the way Apophysis does (LoadLibrary + GetProcAddress by
 * name) and drives it through a short session. Build it for the same
 * architecture as the DLL:
 *
 *   cl /nologo smoke.c                          (MSVC)
 *   x86_64-w64-mingw32-gcc smoke.c -o smoke.exe (MinGW)
 *
 *   smoke.exe path\to\truchet_ms.dll
 */
#include <windows.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

typedef const char *(__cdecl *fn_name)(void);
typedef int (__cdecl *fn_count)(void);
typedef const char *(__cdecl *fn_name_at)(int);
typedef void *(__cdecl *fn_create)(void);
typedef int (__cdecl *fn_destroy)(void **);
typedef int (__cdecl *fn_init)(void *, void *, void *, void *, void *, double);
typedef int (__cdecl *fn_init3d)(void *, void *, void *, void *, void *, void *, void *, double);
typedef int (__cdecl *fn_initdc)(void *, void *, void *, void *, void *, void *, void *, void *,
                                 double, double, double, double, double, double, double);
typedef int (__cdecl *fn_inst)(void *);
typedef int (__cdecl *fn_getvar)(void *, const char *, double *);
typedef int (__cdecl *fn_resetvar)(void *, const char *);

static int failures;
#define CHECK(c)                                                  \
    do {                                                          \
        if (!(c)) {                                               \
            printf("FAIL line %d: %s\n", __LINE__, #c);           \
            failures++;                                           \
        }                                                         \
    } while (0)

static void *must(HMODULE dll, const char *name)
{
    void *p = (void *)GetProcAddress(dll, name);
    if (!p) {
        printf("FAIL missing export %s\n", name);
        failures++;
    }
    return p;
}

static int setvar(fn_getvar set, void *vp, const char *name, double v)
{
    return set(vp, name, &v);
}

int main(int argc, char **argv)
{
    HMODULE dll;
    fn_name get_name;
    fn_count get_count, get_arch;
    fn_name_at name_at;
    fn_create create;
    fn_destroy destroy;
    fn_init init;
    fn_init3d init3d;
    fn_initdc initdc;
    fn_inst prepare, calc;
    fn_getvar get_var, set_var;
    fn_resetvar reset_var;
    void *vp;
    double ftx = 0, fty = 0, ftz = 0, fpx = 0, fpy = 0, fpz = 0, color = 0, value = 0;
    int i, hits;

    if (argc < 2) {
        printf("usage: smoke path\\to\\truchet_ms.dll\n");
        return 2;
    }
    dll = LoadLibraryA(argv[1]);
    if (!dll) {
        printf("FAIL LoadLibrary(%s) error %lu\n", argv[1], GetLastError());
        return 1;
    }

    get_name = (fn_name)must(dll, "PluginVarGetName");
    get_count = (fn_count)must(dll, "PluginVarGetNrVariables");
    name_at = (fn_name_at)must(dll, "PluginVarGetVariableNameAt");
    create = (fn_create)must(dll, "PluginVarCreate");
    destroy = (fn_destroy)must(dll, "PluginVarDestroy");
    init = (fn_init)must(dll, "PluginVarInit");
    init3d = (fn_init3d)must(dll, "PluginVarInit3D");
    initdc = (fn_initdc)must(dll, "PluginVarInitDC");
    prepare = (fn_inst)must(dll, "PluginVarPrepare");
    calc = (fn_inst)must(dll, "PluginVarCalc");
    get_var = (fn_getvar)must(dll, "PluginVarGetVariable");
    set_var = (fn_getvar)must(dll, "PluginVarSetVariable");
    reset_var = (fn_resetvar)must(dll, "PluginVarResetVariable");
    get_arch = (fn_count)must(dll, "PluginVarGetArchitecture");
    if (failures) return 1;

    CHECK(strcmp(get_name(), "truchet_ms") == 0);
    CHECK(get_count() == 12);
    CHECK(strcmp(name_at(0), "truchet_ms_seed") == 0);
    CHECK(strcmp(name_at(99), "") == 0);
    CHECK(get_arch() == (sizeof(void *) == 8 ? 0x40 : 0x20));

    vp = create();
    CHECK(vp != NULL);
    CHECK(get_var(vp, "truchet_ms_width", &value) && value == 6.0);
    CHECK(setvar(set_var, vp, "truchet_ms_width", 1000.0) && get_var(vp, "truchet_ms_width", &value) && value == 64.0);
    CHECK(reset_var(vp, "truchet_ms_width") && get_var(vp, "truchet_ms_width", &value) && value == 6.0);
    CHECK(!setvar(set_var, vp, "no_such_variable", 1.0));

    /* Filter mode: about half of a grid of points lands on ink. */
    CHECK(init3d(vp, &fpx, &fpy, &fpz, &ftx, &fty, &ftz, 1.0));
    CHECK(setvar(set_var, vp, "truchet_ms_mode", 1.0));
    CHECK(prepare(vp));
    hits = 0;
    for (i = 0; i < 1000; i++) {
        ftx = i * 0.137 - 20.0;
        fty = sin(i * 0.071) * 9.0;
        fpx = fpy = fpz = 0.0;
        CHECK(calc(vp));
        if (fpx != 0.0 || fpy != 0.0) hits++;
    }
    CHECK(hits > 100 && hits < 900);

    /* Generate mode, square and rounded sets: nearly every call finds ink. */
    for (i = 0; i < 2; i++) {
        int n, found = 0;
        CHECK(setvar(set_var, vp, "truchet_ms_mode", 0.0));
        CHECK(setvar(set_var, vp, "truchet_ms_style", (double)i));
        CHECK(prepare(vp));
        for (n = 0; n < 1000; n++) {
            ftx = fty = ftz = 0.0;
            fpx = fpy = fpz = 0.0;
            CHECK(calc(vp));
            CHECK(isfinite(fpx) && isfinite(fpy) && fabs(fpx) <= 3.0001 && fabs(fpy) <= 3.0001);
            if (fpx != 0.0 || fpy != 0.0) found++;
        }
        CHECK(found > 800);
    }

    /* The 2D and DirectColor entry points run the same code. */
    CHECK(init(vp, &fpx, &fpy, &ftx, &fty, 1.0));
    CHECK(prepare(vp));
    CHECK(calc(vp));
    CHECK(initdc(vp, &fpx, &fpy, &fpz, &ftx, &fty, &ftz, &color, 1.0, 1, 0, 0, 1, 0, 0));
    CHECK(prepare(vp));
    CHECK(calc(vp));

    CHECK(destroy(&vp) && vp == NULL);
    FreeLibrary(dll);

    if (failures) {
        printf("%d check(s) failed\n", failures);
        return 1;
    }
    printf("ok: %s (%d-bit)\n", argv[1], (int)(sizeof(void *) * 8));
    return 0;
}
