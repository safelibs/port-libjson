#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "json_object.h"
#include "json_tokener.h"

static void test_object_helpers(void)
{
	struct json_object *obj = json_object_new_object();
	struct json_object *arr = json_object_new_array_ext(8);
	struct json_object *first = json_object_new_int(42);
	struct json_object *second;

	assert(obj != NULL);
	assert(arr != NULL);
	assert(first != NULL);

	assert(json_c_object_sizeof() > 0);
	printf("json_c_object_sizeof() returned a non-zero size\n");

	assert(json_object_get_object(obj) != NULL);
	assert(json_object_get_object(arr) == NULL);
	assert(json_object_get_object(NULL) == NULL);
	printf("json_object_get_object() handled object, array, and NULL inputs\n");

	assert(json_object_get_array(arr) != NULL);
	assert(json_object_get_array(obj) == NULL);
	assert(json_object_get_array(NULL) == NULL);
	printf("json_object_get_array() handled array, object, and NULL inputs\n");

	json_object_get(first);
	assert(json_object_object_add_ex(obj, "answer", first, JSON_C_OBJECT_ADD_CONSTANT_KEY) == 0);
	assert(json_object_object_length(obj) == 1);
	assert(json_object_object_get(obj, "answer") == first);
	assert(json_object_get_int(json_object_object_get(obj, "answer")) == 42);
	printf("json_object_object_add_ex() added a constant-key field\n");

	second = json_object_new_int(7);
	assert(second != NULL);
	assert(json_object_object_add_ex(obj, "answer", second, 0) == 0);
	assert(json_object_object_length(obj) == 1);
	assert(json_object_get_int(json_object_object_get(obj, "answer")) == 7);
	assert(json_object_put(first) == 1);
	printf("json_object_object_add_ex() replaced an existing field\n");

	assert(json_object_object_add_ex(
	           obj, "payload", arr, JSON_C_OBJECT_ADD_KEY_IS_NEW | JSON_C_OBJECT_ADD_CONSTANT_KEY) == 0);
	assert(json_object_object_length(obj) == 2);
	assert(json_object_object_get(obj, "payload") == arr);
	assert(json_object_array_add(arr, json_object_new_string("alpha")) == 0);
	assert(json_object_array_add(arr, json_object_new_string("beta")) == 0);
	assert(json_object_object_add_ex(obj, "self", obj, 0) == -1);
	assert(json_object_object_length(obj) == 2);
	printf("json_object_object_add_ex() rejected a trivial self-reference\n");
	printf("object=%s\n", json_object_to_json_string_ext(obj, JSON_C_TO_STRING_PLAIN));

	json_object_put(obj);
}

static void test_tokener_depth(void)
{
	static const char nested[] = "[[1]]";
	static const char shallow[] = "[1]";
	struct json_tokener *tok = json_tokener_new_ex(2);
	struct json_object *parsed;

	assert(tok != NULL);

	parsed = json_tokener_parse_ex(tok, nested, (int)sizeof(nested));
	assert(parsed == NULL);
	assert(json_tokener_get_error(tok) == json_tokener_error_depth);
	printf("json_tokener_new_ex(2) rejected nested input: %s\n",
	       json_tokener_error_desc(json_tokener_get_error(tok)));

	json_tokener_reset(tok);
	parsed = json_tokener_parse_ex(tok, shallow, (int)sizeof(shallow));
	assert(parsed != NULL);
	assert(json_tokener_get_error(tok) == json_tokener_success);
	printf("json_tokener_new_ex(2) parsed shallow input: %s\n",
	       json_object_to_json_string_ext(parsed, JSON_C_TO_STRING_PLAIN));
	json_object_put(parsed);
	json_tokener_free(tok);

	tok = json_tokener_new_ex(4);
	assert(tok != NULL);
	parsed = json_tokener_parse_ex(tok, nested, (int)sizeof(nested));
	assert(parsed != NULL);
	assert(json_tokener_get_error(tok) == json_tokener_success);
	printf("json_tokener_new_ex(4) parsed nested input: %s\n",
	       json_object_to_json_string_ext(parsed, JSON_C_TO_STRING_PLAIN));
	json_object_put(parsed);
	json_tokener_free(tok);
}

int main(void)
{
	test_object_helpers();
	printf("========================================\n");
	test_tokener_depth();
	printf("========================================\n");
	return 0;
}
