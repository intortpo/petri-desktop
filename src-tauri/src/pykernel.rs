use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

const KERNEL_PY: &str = r#"
import sys
g = {}
while True:
    line = sys.stdin.readline()
    if not line:
        break
    line = line.rstrip('\n')
    try:
        if line.startswith('EVAL '):
            r = eval(line[5:], g, g)
            sys.stdout.write('OK ' + repr(r) + '\n')
        elif line.startswith('EXEC '):
            exec(line[5:], g, g)
            sys.stdout.write('OK None\n')
        else:
            sys.stdout.write('ERR unknown\n')
    except Exception as e:
        sys.stdout.write('ERR ' + str(e) + '\n')
    sys.stdout.flush()
"#;

pub struct PyKernel {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl PyKernel {
    pub fn start() -> Result<Self, String> {
        let mut child = Command::new("python3")
            .arg("-u")
            .arg("-c")
            .arg(KERNEL_PY)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("python3 spawn: {e}"))?;
        let stdin = child.stdin.take().ok_or("python stdin")?;
        let stdout = child.stdout.take().ok_or("python stdout")?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    pub fn exec(&mut self, code: &str) -> Result<String, String> {
        self.send("EXEC", code)
    }

    pub fn eval(&mut self, expr: &str) -> Result<String, String> {
        self.send("EVAL", expr)
    }

    fn send(&mut self, op: &str, payload: &str) -> Result<String, String> {
        let line = format!("{op} {payload}\n");
        self.stdin.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())?;
        let mut out = String::new();
        self.stdout.read_line(&mut out).map_err(|e| e.to_string())?;
        if let Some(rest) = out.strip_prefix("OK ") {
            Ok(rest.trim().to_string())
        } else if out.starts_with("OK") {
            Ok("None".into())
        } else {
            Err(out.trim().to_string())
        }
    }
}

impl Drop for PyKernel {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_survives_second_turn() {
        let mut k = match PyKernel::start() {
            Ok(k) => k,
            Err(e) => {
                eprintln!("skip python kernel: {e}");
                return;
            }
        };
        k.exec("mesh_token = 42").unwrap();
        let v = k.eval("mesh_token").unwrap();
        assert_eq!(v, "42");
    }
}
