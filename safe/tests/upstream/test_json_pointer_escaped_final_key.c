#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>

#include "json.h"

int main(void)
{
	struct json_object *actual;
	struct json_object *expected;

	actual = json_tokener_parse("{ }");
	expected = json_tokener_parse("{ 'a~1b': 1, 'm~0n': 8 }");
	assert(0 == json_pointer_set(&actual, "/a~1b", json_object_new_int(1)));
	assert(0 == json_pointer_set(&actual, "/m~0n", json_object_new_int(8)));
	assert(1 == json_object_equal(actual, expected));
	json_object_put(actual);
	json_object_put(expected);

	actual = json_tokener_parse("{ 'outer': { } }");
	expected = json_tokener_parse("{ 'outer': { 'a~1b': 1 } }");
	assert(0 == json_pointer_set(&actual, "/outer/a~1b", json_object_new_int(1)));
	assert(1 == json_object_equal(actual, expected));
	json_object_put(actual);
	json_object_put(expected);

	actual = json_tokener_parse("{ 'a/b': 1, 'a~1b': 2 }");
	expected = json_tokener_parse("{ 'a/b': 1, 'a~1b': 9 }");
	assert(0 == json_pointer_set(&actual, "/a~1b", json_object_new_int(9)));
	assert(1 == json_object_equal(actual, expected));
	json_object_put(actual);
	json_object_put(expected);

	printf("json_pointer escaped final key semantics matched upstream.\n");
	return 0;
}
