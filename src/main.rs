use std::io::stdin;

use serde_json::{json, Map, Value};

fn main() {
    let json: Map<String, Value> = serde_json::from_reader(stdin()).unwrap();
    println!("{}", serde_json::to_string_pretty(&json).unwrap());

    println!("===================");

    let result = flatten(json);
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}

fn flatten(mut json: Map<String, Value>) -> Map<String, Value> {
    let keys: Vec<String> = json.keys().cloned().collect();

    for key in keys {
        if let Some(object) = json[&key].as_object_mut() {
            let object = std::mem::take(object);
            // we don't need the original key anymore
            json.remove_entry(&key);

            insert_object(&mut json, &key, object);
        } else if let Some(array) = json[&key].as_array() {
            let mut array = array.clone();
            let mut delete = true;

            insert_array(&mut json, &key, &mut array, &mut delete);

            if delete {
                json.remove_entry(&key);
            } else {
                // if we can't remove the entry entirely we need to remove all the empty objects and arrays in it
                array.retain(|value| !(value.is_object() || value.is_array()));
                json[&key] = json!(array);
            }
        }
    }

    json
}

fn insert_object(
    base_map: &mut Map<String, Value>,
    base_key: &str,
    to_flatten: Map<String, Value>,
) {
    let mut object = flatten(to_flatten);

    let keys: Vec<String> = object.keys().cloned().collect();

    for key in keys {
        let new_key = format!("{base_key}.{key}");

        // let entry = base_map.entry(&new_key).or_insert(json!([]));

        if let Some(array) = base_map[&key].as_array() {
            let mut array = array.clone();
            let mut delete = true;

            insert_array(base_map, &key, &mut array, &mut delete);

            if delete {
                base_map.remove_entry(&key);
            } else {
                // if we can't remove the entry entirely we need to remove all the empty objects and arrays in it
                array.retain(|value| !(value.is_object() || value.is_array()));
                base_map[&key] = json!(array);
            }
        } else {
            assert!(!object[&new_key].is_object());
            // if there was a collision we take what was in the object
            let v = std::mem::take(&mut object[&new_key]);
            // and insert it back in an array
            if v.is_null() {
                object[&new_key] = json!([object[&key]]);
            } else {
                object[&new_key] = json!([v, object[&key]]);
            }
        }
    }
}

fn insert_array(
    base_map: &mut Map<String, Value>,
    base_key: &str,
    array: &mut Vec<Value>,
    delete: &mut bool,
) {
    for idx in 0..array.len() {
        if let Some(object) = array[idx].as_object_mut() {
            let object = std::mem::take(object);
            insert_object(base_map, base_key, object);
        } else if let Some(sub_array) = array[idx].as_array_mut() {
            let mut sub_array = std::mem::take(sub_array);
            insert_array(base_map, base_key, &mut sub_array, delete);
            println!("Appending {:?}\nto {:?}", sub_array, array);
            array.append(&mut sub_array);
            *delete = false;
        } else {
            *delete = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_flattening() {
        let mut base: Value = json!({
          "id": "287947",
          "title": "Shazam!",
          "release_date": 1553299200,
          "genres": [
            "Action",
            "Comedy",
            "Fantasy"
          ]
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten(json.clone());

        assert_eq!(flat, json);
    }

    #[test]
    fn flatten_object() {
        let mut base: Value = json!({
          "a": {
            "b": "c",
            "d": "e",
            "f": "g"
          }
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten(json.clone());

        assert_eq!(
            &flat,
            json!({
                "a.b": ["c"],
                "a.d": ["e"],
                "a.f": ["g"]
            })
            .as_object()
            .unwrap()
        );
    }

    #[test]
    fn flatten_array() {
        let mut base: Value = json!({
          "a": [
            { "b": "c" },
            { "b": "d" },
            { "b": "e" },
          ]
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten(json.clone());

        assert_eq!(
            &flat,
            json!({
                "a.b": ["c", "d", "e"],
            })
            .as_object()
            .unwrap()
        );

        // here we're supposed to keep 42 in "a"
        let mut base: Value = json!({
          "a": [
            42,
            { "b": "c" },
            { "b": "d" },
            { "b": "e" },
          ]
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten(json.clone());

        assert_eq!(
            &flat,
            json!({
                "a": [42],
                "a.b": ["c", "d", "e"],
            })
            .as_object()
            .unwrap()
        );
    }

    #[test]
    fn collision_with_object() {
        let mut base: Value = json!({
          "a": {
            "b": "c",
          },
          "a.b": "d",
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten(json.clone());

        assert_eq!(
            &flat,
            json!({
                "a.b": ["d", "c"],
            })
            .as_object()
            .unwrap()
        );
    }

    #[test]
    fn collision_with_array() {
        let mut base: Value = json!({
          "a": [
            { "b": "c" },
            { "b": "d", "c": "e" },
            35
          ],
          "a.b": "f",
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten(json.clone());

        assert_eq!(
            &flat,
            json!({
                "a.b": ["f", "c", "d"],
                "a.c": ["e"],
                "a": [35],
            })
            .as_object()
            .unwrap()
        );
    }

    #[test]
    fn flatten_nested_arrays() {
        let mut base: Value = json!({
          "a": [
            ["b", "c"],
            { "d": "e" },
            ["f", "g"],
            [
                { "h": "i" },
                { "d": "j" },
            ],
            ["k", "l"],
          ]
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten(json.clone());

        assert_eq!(
            &flat,
            json!({
                "a": ["b", "c", "f", "g", "k", "l"],
                "a.d": ["e", "j"],
                "a.h": ["i"],
            })
            .as_object()
            .unwrap()
        );
    }

    #[test]
    fn flatten_nested_arrays_and_objects() {
        let mut base: Value = json!({
          "a": [
            "b",
            ["c", "d"],
            { "e": ["f", "g"] },
            [
                { "h": "i" },
                { "e": ["j", { "z": "y" }] },
            ],
            ["l"],
            "m",
          ]
        });
        let json = std::mem::take(base.as_object_mut().unwrap());
        let flat = flatten(json.clone());

        println!("{}", serde_json::to_string_pretty(&flat).unwrap());

        assert_eq!(
            &flat,
            json!({
                "a": ["b", "m", "c", "d", "l"],
                "a.e": ["f", "g", "j"],
                "a.h": ["i"],
                "a.e.z": ["y"],
            })
            .as_object()
            .unwrap()
        );
    }
}
