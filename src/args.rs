use anyhow::{anyhow, Result};

pub const SEP: &str = ":::";
pub type ArgGroup = Vec<String>;

/// Returns (common_args, instance_groups)
pub fn split_args(n: usize, args: &[String]) -> Result<(Vec<String>, Vec<ArgGroup>)> {
    if n == 0 {
        return Err(anyhow!("--n must be >= 1"));
    }

    if let Some(first_sep) = args.iter().position(|a| a == SEP) {
        let common = args[..first_sep].to_vec();

        let mut groups: Vec<ArgGroup> = Vec::new();
        let mut current: ArgGroup = Vec::new();

        for tok in &args[first_sep + 1..] {
            if tok == SEP {
                groups.push(current);
                current = Vec::new();
            } else {
                current.push(tok.clone());
            }
        }
        groups.push(current);

        if groups.len() != n {
            return Err(anyhow!(
                "Got {} group(s) after '{SEP}', but --n is {n}. \
                 Provide exactly {n} groups separated by '{SEP}'.",
                groups.len()
            ));
        }

        Ok((common, groups))
    } else {
        if args.len() < n {
            return Err(anyhow!(
                "Not enough args for shorthand mode: need at least {n} instance arg(s). \
                 Either add args or use '{SEP}' grouping."
            ));
        }

        let split_at = args.len() - n;
        let common = args[..split_at].to_vec();
        let groups = args[split_at..]
            .iter()
            .cloned()
            .map(|a| vec![a])
            .collect();

        Ok((common, groups))
    }
}