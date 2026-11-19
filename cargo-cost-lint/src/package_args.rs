/// Build the `--package <name>` / `--workspace` argument list for cargo,
/// validating each name against the workspace members and deduplicating
/// repeated names while preserving first-seen order.
pub fn validate_and_build_package_args(
    packages: &[String],
    workspace: bool,
    available_packages: &[String],
) -> Result<Vec<String>, String> {
    if workspace && !packages.is_empty() {
        return Err(
            "Error: The argument '--workspace' cannot be used with '--package <SPEC>'".to_string(),
        );
    }

    if workspace {
        return Ok(vec!["--workspace".to_string()]);
    }

    if packages.is_empty() {
        return Ok(Vec::new());
    }

    for pkg in packages {
        if !available_packages.iter().any(|p| p == pkg) {
            let valid = available_packages.join(", ");
            return Err(format!(
                "Error: Package '{}' not found in workspace. Valid workspace members are: {}",
                pkg, valid
            ));
        }
    }

    // Deduplicate while preserving first-seen order: a caller that passes
    // `--package contract_a --package contract_a` should produce a single
    // `--package contract_a` flag.  Using a Vec + linear scan is fine here
    // because the number of workspace members is small.
    let mut seen: Vec<&str> = Vec::with_capacity(packages.len());
    for pkg in packages {
        if !seen.contains(&pkg.as_str()) {
            seen.push(pkg.as_str());
        }
    }

    let mut args = Vec::new();
    for pkg in seen {
        args.push("--package".to_string());
        args.push(pkg.to_string());
    }
    Ok(args)
}
