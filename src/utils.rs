use ethabi::Token;

fn print_with_indentation(indent: usize, s: &str) {
    for _i in 0..indent {
        print!("    ");
    }
    println!("{}", s);
}

/// Recursively prints nested tokens with increasing identation
pub fn print_parse_tree(parse_tree: &Token, indentation: usize) {
    match parse_tree {
        Token::Array(ref elements) => {
            print_with_indentation(indentation, "Array: ");
            for item in elements {
                print_parse_tree(item, indentation + 1);
            }
        }
        Token::Tuple(ref elements) => {
            print_with_indentation(indentation, "Tuple: ");
            for item in elements {
                print_parse_tree(item, indentation + 1);
            }
        }
        // Avoid normal bytes debug output which prints a huge array of bytes
        Token::Bytes(ref bytes) => {
            print_with_indentation(indentation, &format!("Bytes: {:?}", hex::encode(bytes)));
        }
        token => {
            print_with_indentation(indentation, &format!("{:?}", token));
        }
    }
}
