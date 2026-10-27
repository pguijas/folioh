/** Functions module for testing. */

/**
 * Greets a person by name.
 * @param {string} name - The name to greet.
 * @param {boolean} [excited=false] - Whether to be excited.
 * @returns {string} The greeting message.
 * @throws {Error} If name is empty.
 * @deprecated Use greetV2 instead.
 * @example
 * greet("world")
 */
export function greet(name, excited = false) {
  return excited ? name + "!" : name;
}

/**
 * Fetches data from a URL.
 * @param {string} url - The endpoint.
 * @param {...any} options - Fetch options.
 * @returns {Promise} The response.
 */
export async function fetchData(url, ...options) {
  return fetch(url, ...options);
}

/**
 * Yields integers in a range.
 * @yields {number} Each integer.
 * @param {number} start - Start value.
 * @param {number} end - End value (exclusive).
 */
export function* range(start, end) {
  for (let i = start; i < end; i++) yield i;
}
