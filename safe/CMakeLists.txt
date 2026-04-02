cmake_minimum_required(VERSION 3.9)

cmake_policy(SET CMP0048 NEW)
project(json-c LANGUAGES C VERSION 0.17)
cmake_policy(SET CMP0038 NEW)
cmake_policy(SET CMP0042 NEW)
cmake_policy(SET CMP0054 NEW)

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE debug)
endif()

set(CMAKE_C_FLAGS_RELEASE "${CMAKE_C_FLAGS_RELEASE} -O2")

if(POLICY CMP0075)
    cmake_policy(SET CMP0075 NEW)
endif()

include(CTest)

set(CPACK_PACKAGE_NAME "${PROJECT_NAME}")
set(CPACK_PACKAGE_VERSION_MAJOR "${PROJECT_VERSION_MAJOR}")
set(CPACK_PACKAGE_VERSION_MINOR "${PROJECT_VERSION_MINOR}")
set(CPACK_PACKAGE_VERSION_PATCH "${PROJECT_VERSION_PATCH}")
set(JSON_C_BUGREPORT "json-c@googlegroups.com")

include(CheckCSourceCompiles)
include(CheckIncludeFile)
include(CheckIncludeFiles)
include(CheckSymbolExists)
include(CheckTypeSize)
include(CMakePackageConfigHelpers)
include(CPack)
include(GNUInstallDirs)

option(BUILD_SHARED_LIBS "Default to building shared libraries" ON)
option(BUILD_STATIC_LIBS "Default to building static libraries" ON)
option(DISABLE_BSYMBOLIC "Avoid linking with -Bsymbolic-function." OFF)
option(DISABLE_THREAD_LOCAL_STORAGE "Disable using Thread-Local Storage (HAVE___THREAD)." OFF)
option(DISABLE_WERROR "Avoid treating compiler warnings as fatal errors." OFF)
option(ENABLE_RDRAND "Enable RDRAND Hardware RNG Hash Seed." OFF)
option(ENABLE_THREADING "Enable partial threading support." OFF)
option(OVERRIDE_GET_RANDOM_SEED "Override json_c_get_random_seed() with custom code." OFF)
option(DISABLE_EXTRA_LIBS "Avoid linking against extra libraries, such as libbsd." OFF)
option(DISABLE_JSON_POINTER "Disable JSON pointer (RFC6901) and JSON patch support." OFF)
option(DISABLE_JSON_PATCH "Disable JSON patch (RFC6902) support." OFF)
option(NEWLOCALE_NEEDS_FREELOCALE "Work around newlocale bugs in old FreeBSD by calling freelocale" OFF)
option(BUILD_APPS "Default to building apps" ON)

if(BUILD_SHARED_LIBS)
    add_definitions(-D JSON_C_DLL)
endif()

if(UNIX OR MINGW OR CYGWIN)
    list(APPEND CMAKE_REQUIRED_DEFINITIONS -D_GNU_SOURCE)
endif()

if(UNIX)
    list(APPEND CMAKE_REQUIRED_LIBRARIES m)
endif()

if(MSVC)
    list(APPEND CMAKE_REQUIRED_DEFINITIONS /D_CRT_SECURE_NO_DEPRECATE)
    list(APPEND CMAKE_REQUIRED_FLAGS /wd4996)
endif()

if(NOT DISABLE_STATIC_FPIC)
    set(CMAKE_POSITION_INDEPENDENT_CODE ON)
endif()

check_include_file("fcntl.h" HAVE_FCNTL_H)
check_include_file("inttypes.h" HAVE_INTTYPES_H)
check_include_file(stdarg.h HAVE_STDARG_H)
check_include_file(strings.h HAVE_STRINGS_H)
check_include_file(string.h HAVE_STRING_H)
check_include_file(syslog.h HAVE_SYSLOG_H)
check_include_files("stdlib.h;stdarg.h;string.h;float.h" STDC_HEADERS)
check_include_file(unistd.h HAVE_UNISTD_H)
check_include_file(sys/types.h HAVE_SYS_TYPES_H)
check_include_file(sys/resource.h HAVE_SYS_RESOURCE_H)
check_include_file("dlfcn.h" HAVE_DLFCN_H)
check_include_file("endian.h" HAVE_ENDIAN_H)
check_include_file("limits.h" HAVE_LIMITS_H)
check_include_file("locale.h" HAVE_LOCALE_H)
check_include_file("memory.h" HAVE_MEMORY_H)
check_include_file(stdint.h HAVE_STDINT_H)
check_include_file(stdlib.h HAVE_STDLIB_H)
check_include_file(sys/cdefs.h HAVE_SYS_CDEFS_H)
check_include_file(sys/param.h HAVE_SYS_PARAM_H)
check_include_file(sys/random.h HAVE_SYS_RANDOM_H)
check_include_file(sys/stat.h HAVE_SYS_STAT_H)
check_include_file(xlocale.h HAVE_XLOCALE_H)

if(HAVE_INTTYPES_H)
    set(JSON_C_HAVE_INTTYPES_H 1)
endif()
if(HAVE_STDINT_H)
    set(JSON_C_HAVE_STDINT_H 1)
endif()

check_symbol_exists(_isnan "float.h" HAVE_DECL__ISNAN)
check_symbol_exists(_finite "float.h" HAVE_DECL__FINITE)

if((MSVC AND NOT (MSVC_VERSION LESS 1800)) OR MINGW OR CYGWIN OR UNIX)
    check_symbol_exists(INFINITY "math.h" HAVE_DECL_INFINITY)
    check_symbol_exists(isinf "math.h" HAVE_DECL_ISINF)
    check_symbol_exists(isnan "math.h" HAVE_DECL_ISNAN)
    check_symbol_exists(nan "math.h" HAVE_DECL_NAN)
endif()

check_symbol_exists(_doprnt "stdio.h" HAVE_DOPRNT)
if(UNIX OR MINGW OR CYGWIN)
    check_symbol_exists(snprintf "stdio.h" HAVE_SNPRINTF)
endif()
check_symbol_exists(vasprintf "stdio.h" HAVE_VASPRINTF)
check_symbol_exists(vsnprintf "stdio.h" HAVE_VSNPRINTF)
check_symbol_exists(vprintf "stdio.h" HAVE_VPRINTF)

check_symbol_exists(arc4random "stdlib.h" HAVE_ARC4RANDOM)
if(NOT HAVE_ARC4RANDOM AND DISABLE_EXTRA_LIBS STREQUAL "OFF")
    check_include_file(bsd/stdlib.h HAVE_BSD_STDLIB_H)
    if(HAVE_BSD_STDLIB_H)
        list(APPEND CMAKE_REQUIRED_LIBRARIES bsd)
        unset(HAVE_ARC4RANDOM CACHE)
        check_symbol_exists(arc4random "bsd/stdlib.h" HAVE_ARC4RANDOM)
        if(NOT HAVE_ARC4RANDOM)
            list(REMOVE_ITEM CMAKE_REQUIRED_LIBRARIES bsd)
        endif()
    endif()
endif()

if(HAVE_FCNTL_H)
    check_symbol_exists(open "fcntl.h" HAVE_OPEN)
endif()
if(HAVE_STDLIB_H)
    check_symbol_exists(realloc "stdlib.h" HAVE_REALLOC)
endif()
if(HAVE_LOCALE_H)
    check_symbol_exists(setlocale "locale.h" HAVE_SETLOCALE)
    check_symbol_exists(uselocale "locale.h" HAVE_USELOCALE)
endif()

if(HAVE_STRINGS_H)
    check_symbol_exists(strcasecmp "strings.h" HAVE_STRCASECMP)
    check_symbol_exists(strncasecmp "strings.h" HAVE_STRNCASECMP)
endif()
if(HAVE_STRING_H)
    check_symbol_exists(strdup "string.h" HAVE_STRDUP)
    check_symbol_exists(strerror "string.h" HAVE_STRERROR)
endif()
if(HAVE_SYSLOG_H)
    check_symbol_exists(vsyslog "syslog.h" HAVE_VSYSLOG)
endif()
if(HAVE_SYS_RANDOM_H)
    check_symbol_exists(getrandom "sys/random.h" HAVE_GETRANDOM)
endif()
if(HAVE_SYS_RESOURCE_H)
    check_symbol_exists(getrusage "sys/resource.h" HAVE_GETRUSAGE)
endif()

check_symbol_exists(strtoll "stdlib.h" HAVE_STRTOLL)
check_symbol_exists(strtoull "stdlib.h" HAVE_STRTOULL)

set(json_c_strtoll "strtoll")
if(NOT HAVE_STRTOLL)
    check_symbol_exists(_strtoi64 "stdlib.h" __have_strtoi64)
    if(__have_strtoi64)
        set(json_c_strtoll "_strtoi64")
    endif()
endif()

set(json_c_strtoull "strtoull")
if(NOT HAVE_STRTOULL)
    check_symbol_exists(_strtoui64 "stdlib.h" __have_strtoui64)
    if(__have_strtoui64)
        set(json_c_strtoull "_strtoui64")
    endif()
endif()

check_type_size(int SIZEOF_INT)
check_type_size(int64_t SIZEOF_INT64_T)
check_type_size(long SIZEOF_LONG)
check_type_size("long long" SIZEOF_LONG_LONG)
check_type_size("size_t" SIZEOF_SIZE_T)
if(MSVC)
    list(APPEND CMAKE_EXTRA_INCLUDE_FILES BaseTsd.h)
    check_type_size("SSIZE_T" SIZEOF_SSIZE_T)
else()
    check_type_size("ssize_t" SIZEOF_SSIZE_T)
endif()

check_c_source_compiles(
"
extern void json_object_get();
__asm__(\".section .gnu.json_object_get\\n\\t.ascii \\\"Please link against libjson-c instead of libjson\\\"\\n\\t.text\");
int main(int c, char *v) { return 0;}
"
HAS_GNU_WARNING_LONG)

check_c_source_compiles(
"int main() { int i, x = 0; i = __sync_add_and_fetch(&x,1); return x; }"
HAVE_ATOMIC_BUILTINS)

if(NOT DISABLE_THREAD_LOCAL_STORAGE)
    check_c_source_compiles(
        "__thread int x = 0; int main() { return 0; }"
        HAVE___THREAD)
    if(HAVE___THREAD)
        set(SPEC___THREAD __thread)
    elseif(MSVC)
        set(SPEC___THREAD __declspec(thread))
    endif()
endif()

if(WIN32)
    set(ENABLE_RDRAND 0)
endif()

set(HAVE_JSON_TOKENER_GET_PARSE_END 1)

configure_file(${CMAKE_CURRENT_SOURCE_DIR}/cmake/config.h.in ${PROJECT_BINARY_DIR}/config.h)
configure_file(${CMAKE_CURRENT_SOURCE_DIR}/cmake/json_config.h.in ${PROJECT_BINARY_DIR}/json_config.h)
configure_file(${CMAKE_CURRENT_SOURCE_DIR}/apps/cmake/apps_config.h.in ${PROJECT_BINARY_DIR}/apps_config.h)

if(NOT ("${CMAKE_C_COMPILER_ID}" STREQUAL "MSVC"))
    check_c_source_compiles(
"
#define _REENTRANT 1
#include <sys/types.h>
int main(void) { return 0; }
"
REENTRANT_WORKS)
    if(REENTRANT_WORKS)
        add_compile_options("-D_REENTRANT")
    endif()

    file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/check-version-script.sym" "TEST { global: *; };")
    list(APPEND CMAKE_REQUIRED_LIBRARIES "-Wl,--version-script,${CMAKE_CURRENT_BINARY_DIR}/check-version-script.sym")
    check_c_source_compiles(
"
int main(void) { return 0; }
"
VERSION_SCRIPT_WORKS)
    list(REMOVE_ITEM CMAKE_REQUIRED_LIBRARIES "-Wl,--version-script,${CMAKE_CURRENT_BINARY_DIR}/check-version-script.sym")
endif()

set(JSON_C_SOURCE_HEADERS
    ${PROJECT_SOURCE_DIR}/include/json-c/arraylist.h
    ${PROJECT_SOURCE_DIR}/include/json-c/debug.h
    ${PROJECT_SOURCE_DIR}/include/json-c/json_c_version.h
    ${PROJECT_SOURCE_DIR}/include/json-c/json_inttypes.h
    ${PROJECT_SOURCE_DIR}/include/json-c/json_object.h
    ${PROJECT_SOURCE_DIR}/include/json-c/json_object_iterator.h
    ${PROJECT_SOURCE_DIR}/include/json-c/json_tokener.h
    ${PROJECT_SOURCE_DIR}/include/json-c/json_types.h
    ${PROJECT_SOURCE_DIR}/include/json-c/json_util.h
    ${PROJECT_SOURCE_DIR}/include/json-c/json_visit.h
    ${PROJECT_SOURCE_DIR}/include/json-c/linkhash.h
    ${PROJECT_SOURCE_DIR}/include/json-c/printbuf.h
)

if(NOT DISABLE_JSON_POINTER)
    list(APPEND JSON_C_SOURCE_HEADERS ${PROJECT_SOURCE_DIR}/include/json-c/json_pointer.h)
    set(JSON_H_JSON_POINTER "#include \"json_pointer.h\"")
    if(NOT DISABLE_JSON_PATCH)
        list(APPEND JSON_C_SOURCE_HEADERS ${PROJECT_SOURCE_DIR}/include/json-c/json_patch.h)
        set(JSON_H_JSON_PATCH "#include \"json_patch.h\"")
    else()
        set(JSON_H_JSON_PATCH "")
    endif()
else()
    set(JSON_H_JSON_POINTER "")
    set(JSON_H_JSON_PATCH "")
endif()

configure_file(${CMAKE_CURRENT_SOURCE_DIR}/../original/json.h.cmakein ${PROJECT_BINARY_DIR}/json.h @ONLY)

string(TOLOWER "${CMAKE_BUILD_TYPE}" CMAKE_BUILD_TYPE_LOWER)
if(CMAKE_BUILD_TYPE_LOWER STREQUAL "debug")
    set(CARGO_PROFILE "debug")
else()
    set(CARGO_PROFILE "release")
endif()

set(RUST_TARGET_DIR "${PROJECT_BINARY_DIR}/cargo-target")
execute_process(
    COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tools/collect-native-static-libs.sh
            ${CMAKE_CURRENT_SOURCE_DIR}
            ${PROJECT_BINARY_DIR}
            ${RUST_TARGET_DIR}
            ${CARGO_PROFILE}
    RESULT_VARIABLE COLLECT_NATIVE_STATIC_LIBS_RESULT
)
if(NOT COLLECT_NATIVE_STATIC_LIBS_RESULT EQUAL 0)
    message(FATAL_ERROR "collect-native-static-libs.sh failed during configure")
endif()
include(${PROJECT_BINARY_DIR}/rust-native-static-libs.cmake)

add_custom_command(
    OUTPUT
        ${RUST_STATICLIB}
        ${PROJECT_BINARY_DIR}/rust-native-static-libs.cmake
        ${PROJECT_BINARY_DIR}/rust-native-static-libs.pcflags
        ${PROJECT_BINARY_DIR}/rust-native-static-libs.txt
    COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tools/collect-native-static-libs.sh
            ${CMAKE_CURRENT_SOURCE_DIR}
            ${PROJECT_BINARY_DIR}
            ${RUST_TARGET_DIR}
            ${CARGO_PROFILE}
    DEPENDS
        ${PROJECT_SOURCE_DIR}/Cargo.toml
        ${PROJECT_SOURCE_DIR}/.cargo/config.toml
        ${PROJECT_SOURCE_DIR}/abi/public-api-manifest.tsv
        ${PROJECT_SOURCE_DIR}/build.rs
        ${PROJECT_SOURCE_DIR}/src/lib.rs
        ${PROJECT_SOURCE_DIR}/src/abi.rs
        ${PROJECT_SOURCE_DIR}/src/arraylist.rs
        ${PROJECT_SOURCE_DIR}/src/debug.rs
        ${PROJECT_SOURCE_DIR}/src/errors.rs
        ${PROJECT_SOURCE_DIR}/src/iterators.rs
        ${PROJECT_SOURCE_DIR}/src/locale.rs
        ${PROJECT_SOURCE_DIR}/src/linkhash.rs
        ${PROJECT_SOURCE_DIR}/src/mini_json.rs
        ${PROJECT_SOURCE_DIR}/src/numeric.rs
        ${PROJECT_SOURCE_DIR}/src/object.rs
        ${PROJECT_SOURCE_DIR}/src/printbuf.rs
        ${PROJECT_SOURCE_DIR}/src/random_seed.rs
        ${PROJECT_SOURCE_DIR}/src/serializer.rs
        ${PROJECT_SOURCE_DIR}/src/strerror.rs
        ${PROJECT_SOURCE_DIR}/src/stubs.rs
        ${PROJECT_SOURCE_DIR}/src/tokener.rs
        ${PROJECT_SOURCE_DIR}/src/utf8.rs
        ${PROJECT_SOURCE_DIR}/src/variadic.c
        ${PROJECT_SOURCE_DIR}/src/version.rs
        ${PROJECT_SOURCE_DIR}/src/visit.rs
        ${PROJECT_SOURCE_DIR}/tools/collect-native-static-libs.sh
    VERBATIM
)
add_custom_target(rust-staticlib ALL DEPENDS
    ${RUST_STATICLIB}
    ${PROJECT_BINARY_DIR}/rust-native-static-libs.cmake
    ${PROJECT_BINARY_DIR}/rust-native-static-libs.pcflags
    ${PROJECT_BINARY_DIR}/rust-native-static-libs.txt
)

add_custom_command(
    OUTPUT ${PROJECT_BINARY_DIR}/libjson-c.a
    COMMAND ${CMAKE_COMMAND} -E copy_if_different ${RUST_STATICLIB} ${PROJECT_BINARY_DIR}/libjson-c.a
    DEPENDS ${RUST_STATICLIB}
    VERBATIM
)
add_custom_target(json-c-static ALL DEPENDS ${PROJECT_BINARY_DIR}/libjson-c.a)

file(WRITE ${PROJECT_BINARY_DIR}/json-c-empty.c "void json_c_safe_wrapper_anchor(void) {}\n")

if(BUILD_SHARED_LIBS)
    add_library(${PROJECT_NAME} SHARED ${PROJECT_BINARY_DIR}/json-c-empty.c)
    add_dependencies(${PROJECT_NAME} rust-staticlib)
    set_target_properties(${PROJECT_NAME} PROPERTIES
        LINK_DEPENDS "${RUST_STATICLIB}"
        VERSION 5.3.0
        SOVERSION 5
    )
    target_include_directories(${PROJECT_NAME}
        PUBLIC
            $<BUILD_INTERFACE:${PROJECT_SOURCE_DIR}/include/json-c>
            $<BUILD_INTERFACE:${PROJECT_BINARY_DIR}>
    )
    target_link_libraries(${PROJECT_NAME} PUBLIC ${CMAKE_REQUIRED_LIBRARIES} ${RUST_NATIVE_STATIC_LIBS})
    set_property(TARGET ${PROJECT_NAME} APPEND_STRING PROPERTY
        LINK_FLAGS " -Wl,--whole-archive,${RUST_STATICLIB},--no-whole-archive")
    if(VERSION_SCRIPT_WORKS)
        set_property(TARGET ${PROJECT_NAME} APPEND_STRING PROPERTY
            LINK_FLAGS " -Wl,--version-script,${CMAKE_CURRENT_SOURCE_DIR}/../original/json-c.sym")
    endif()
endif()

set(JSON_C_PUBLIC_HEADERS
    ${PROJECT_BINARY_DIR}/json_config.h
    ${PROJECT_BINARY_DIR}/json.h
    ${JSON_C_SOURCE_HEADERS}
)

set(JSON_C_IMPORTED_LINK_LIBRARIES ${CMAKE_REQUIRED_LIBRARIES} ${RUST_NATIVE_STATIC_LIBS})
set(JSON_C_IMPORTED_INCLUDE_DIRS "${_IMPORT_PREFIX}/include;${_IMPORT_PREFIX}/include/json-c")
string(REPLACE ";" " " RUST_NATIVE_STATIC_PCFLAGS_ESCAPED "${RUST_NATIVE_STATIC_PCFLAGS}")

set(JSON_C_TARGETS_FILE "${PROJECT_BINARY_DIR}/${PROJECT_NAME}-targets.cmake")
file(WRITE "${JSON_C_TARGETS_FILE}" [=[
# Generated by the safe json-c wrapper.
get_filename_component(_IMPORT_PREFIX "${CMAKE_CURRENT_LIST_FILE}" PATH)
get_filename_component(_IMPORT_PREFIX "${_IMPORT_PREFIX}" PATH)
get_filename_component(_IMPORT_PREFIX "${_IMPORT_PREFIX}" PATH)
get_filename_component(_IMPORT_PREFIX "${_IMPORT_PREFIX}" PATH)
if(_IMPORT_PREFIX STREQUAL "/")
  set(_IMPORT_PREFIX "")
endif()

if(NOT TARGET json-c::json-c)
  add_library(json-c::json-c SHARED IMPORTED)
  set_target_properties(json-c::json-c PROPERTIES
    IMPORTED_LOCATION "${_IMPORT_PREFIX}/lib/libjson-c.so.5.3.0"
    IMPORTED_SONAME "libjson-c.so.5"
    INTERFACE_INCLUDE_DIRECTORIES "${_IMPORT_PREFIX}/include;${_IMPORT_PREFIX}/include/json-c"
    INTERFACE_LINK_LIBRARIES "@JSON_C_IMPORTED_LINK_LIBRARIES@"
  )
endif()

if(NOT TARGET json-c::json-c-static)
  add_library(json-c::json-c-static STATIC IMPORTED)
  set_target_properties(json-c::json-c-static PROPERTIES
    IMPORTED_LOCATION "${_IMPORT_PREFIX}/lib/libjson-c.a"
    IMPORTED_LINK_INTERFACE_LANGUAGES "C"
    INTERFACE_INCLUDE_DIRECTORIES "${_IMPORT_PREFIX}/include;${_IMPORT_PREFIX}/include/json-c"
    INTERFACE_LINK_LIBRARIES "@JSON_C_IMPORTED_LINK_LIBRARIES@"
  )
endif()
]=])
configure_file("${JSON_C_TARGETS_FILE}" "${JSON_C_TARGETS_FILE}.configured" @ONLY)
file(RENAME "${JSON_C_TARGETS_FILE}.configured" "${JSON_C_TARGETS_FILE}")

configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/Config.cmake.in"
    ${PROJECT_BINARY_DIR}/${PROJECT_NAME}-config.cmake
    INSTALL_DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}"
)

if(UNIX OR MINGW OR CYGWIN)
    set(prefix ${CMAKE_INSTALL_PREFIX})
    set(exec_prefix ${CMAKE_INSTALL_PREFIX})
    set(libdir ${CMAKE_INSTALL_FULL_LIBDIR})
    set(includedir ${CMAKE_INSTALL_FULL_INCLUDEDIR})
    set(VERSION ${PROJECT_VERSION})
    set(LIBS_LIST ${CMAKE_REQUIRED_LIBRARIES} ${RUST_NATIVE_STATIC_LIBS})
    function(list_transform_prepend var prefix)
        set(temp "")
        foreach(f ${${var}})
            list(APPEND temp "${prefix}${f}")
        endforeach()
        set(${var} "${temp}" PARENT_SCOPE)
    endfunction()
    list_transform_prepend(LIBS_LIST "-l")
    string(REPLACE ";" " " LIBS "${LIBS_LIST}")
    configure_file(${CMAKE_CURRENT_SOURCE_DIR}/json-c.pc.in ${PROJECT_BINARY_DIR}/json-c.pc @ONLY)
    set(INSTALL_PKGCONFIG_DIR "${CMAKE_INSTALL_LIBDIR}/pkgconfig" CACHE PATH "Installation directory for pkgconfig (.pc) files")
endif()

if(BUILD_SHARED_LIBS)
    install(TARGETS ${PROJECT_NAME}
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
        INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR} ${CMAKE_INSTALL_INCLUDEDIR}/json-c
    )
endif()

if(BUILD_STATIC_LIBS)
    install(FILES ${PROJECT_BINARY_DIR}/libjson-c.a DESTINATION ${CMAKE_INSTALL_LIBDIR})
endif()

install(FILES ${PROJECT_BINARY_DIR}/${PROJECT_NAME}-config.cmake DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME})
install(FILES ${JSON_C_TARGETS_FILE} DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME})
install(FILES ${PROJECT_BINARY_DIR}/json-c.pc DESTINATION "${INSTALL_PKGCONFIG_DIR}")
install(FILES ${JSON_C_PUBLIC_HEADERS} DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/json-c)

if(CMAKE_PROJECT_NAME STREQUAL PROJECT_NAME AND BUILD_TESTING)
    set(OBJECT_CORE_TEST_NAMES
        test_compare
        test_deep_copy
        test_double_serializer
        test_int_add
        test_int_get
        test_null
        test_object_iterator
        test_set_serializer
        test_set_value
        test_visit
    )
    string(REPLACE ";" "|" OBJECT_CORE_TEST_REGEX_BODY "${OBJECT_CORE_TEST_NAMES}")
    set(OBJECT_CORE_TEST_REGEX "^(${OBJECT_CORE_TEST_REGEX_BODY})$")
    string(REPLACE ";" " " OBJECT_CORE_TEST_NAMES_SH "${OBJECT_CORE_TEST_NAMES}")

    set(PARSER_TEST_NAMES
        test4
        test_cast
        test_charcase
        test_locale
        test_null
        test_parse
        test_parse_int64
        test_public_api
    )
    string(REPLACE ";" "|" PARSER_TEST_REGEX_BODY "${PARSER_TEST_NAMES}")
    set(PARSER_TEST_REGEX "^(${PARSER_TEST_REGEX_BODY})$")

    add_executable(foundation_smoke tests/foundation/foundation_smoke.c)
    add_dependencies(foundation_smoke json-c-static)
    target_include_directories(foundation_smoke
        PUBLIC
            ${PROJECT_SOURCE_DIR}/include/json-c
            ${PROJECT_BINARY_DIR}
    )
    target_link_libraries(foundation_smoke
        PRIVATE
            ${PROJECT_BINARY_DIR}/libjson-c.a
            ${CMAKE_REQUIRED_LIBRARIES}
            ${RUST_NATIVE_STATIC_LIBS}
    )

    add_test(NAME verify_02_foundation_scripts COMMAND foundation_smoke)
    set_tests_properties(verify_02_foundation_scripts PROPERTIES WORKING_DIRECTORY ${PROJECT_BINARY_DIR})
    add_test(NAME foundation_smoke COMMAND foundation_smoke)
    set_tests_properties(foundation_smoke PROPERTIES WORKING_DIRECTORY ${PROJECT_BINARY_DIR})

    add_test(NAME verify_02_layout_check COMMAND ${PROJECT_SOURCE_DIR}/tools/check-layout.sh ${PROJECT_BINARY_DIR})
    set_tests_properties(verify_02_layout_check PROPERTIES WORKING_DIRECTORY ${PROJECT_BINARY_DIR})
    add_test(NAME abi_layout COMMAND ${PROJECT_SOURCE_DIR}/tools/check-layout.sh ${PROJECT_BINARY_DIR})
    set_tests_properties(abi_layout PROPERTIES WORKING_DIRECTORY ${PROJECT_BINARY_DIR})

    add_subdirectory(tests/upstream ${PROJECT_BINARY_DIR}/tests)

    add_test(
        NAME verify_03_object_suite
        COMMAND ${PROJECT_SOURCE_DIR}/tools/run-upstream-tests.sh
                ${PROJECT_BINARY_DIR}
                ${OBJECT_CORE_TEST_REGEX}
    )
    set_tests_properties(
        verify_03_object_suite
        PROPERTIES
            WORKING_DIRECTORY ${PROJECT_BINARY_DIR}
    )

    add_test(
        NAME verify_03_object_relink
        COMMAND ${CMAKE_COMMAND} -E env
                PROJECT_SOURCE_DIR=${PROJECT_SOURCE_DIR}
                PROJECT_BINARY_DIR=${PROJECT_BINARY_DIR}
                OBJECT_CORE_TEST_NAMES=${OBJECT_CORE_TEST_NAMES_SH}
                bash -lc
                "set -euo pipefail; \
                 \"$PROJECT_SOURCE_DIR/tools/relink-original-tests.sh\" \
                     \"$PROJECT_BINARY_DIR/libjson-c.so.5.3.0\" \
                     \"$PROJECT_BINARY_DIR/relinked/tests\"; \
                 for test_name in $OBJECT_CORE_TEST_NAMES; do \
                     srcdir=\"$PROJECT_SOURCE_DIR/tests/upstream\" \
                     top_builddir=\"$PROJECT_BINARY_DIR/relinked\" \
                     \"$PROJECT_SOURCE_DIR/tests/upstream/$test_name.test\"; \
                 done"
    )
    set_tests_properties(
        verify_03_object_relink
        PROPERTIES
            WORKING_DIRECTORY ${PROJECT_BINARY_DIR}
    )

    add_test(
        NAME verify_04_parser_suite
        COMMAND ${PROJECT_SOURCE_DIR}/tools/run-upstream-tests.sh
                ${PROJECT_BINARY_DIR}
                ${PARSER_TEST_REGEX}
    )
    set_tests_properties(
        verify_04_parser_suite
        PROPERTIES
            WORKING_DIRECTORY ${PROJECT_BINARY_DIR}
    )

    if(BUILD_APPS)
        add_test(
            NAME verify_04_tokener_check
            COMMAND ${CMAKE_COMMAND} -E env
                    PROJECT_SOURCE_DIR=${PROJECT_SOURCE_DIR}
                    PROJECT_BINARY_DIR=${PROJECT_BINARY_DIR}
                    bash -lc
                    "set -euo pipefail; \
                     \"$PROJECT_SOURCE_DIR/tools/check-layout.sh\" \
                         \"$PROJECT_BINARY_DIR\"; \
                     \"$PROJECT_BINARY_DIR/apps/json_parse\" -n \
                         \"$PROJECT_SOURCE_DIR/tests/upstream/valid.json\" \
                         >/dev/null"
        )
        set_tests_properties(
            verify_04_tokener_check
            PROPERTIES
                WORKING_DIRECTORY ${PROJECT_BINARY_DIR}
        )
    endif()
endif()

if(BUILD_APPS)
    add_subdirectory(apps ${PROJECT_BINARY_DIR}/apps)
endif()
