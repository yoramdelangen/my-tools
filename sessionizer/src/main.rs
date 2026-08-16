use std::{
    env::args,
    fs,
    path::{self, PathBuf},
    process::Command,
};

fn main() {
    let mut paths: Vec<PathBuf> = Vec::new();
    args()
        .skip(1)
        .into_iter()
        .map(|p| path::absolute(p))
        .filter(|p| p.is_ok())
        .map(|p| p.unwrap())
        .for_each(|loc| {
            println!("Read dir: {:?}", loc);
            let entries = fs::read_dir(&loc)
                .expect(format!("Dir does not exists: {:?}", loc.as_os_str()).as_str());

            for f in entries {
                let mut p = f.as_ref().unwrap().path();
                if !p.is_dir() {
                    continue;
                }

                p.push(".wtp.yml");
                if p.exists() {
                    p.pop();
                    ask_wtp_list(&mut p, &mut paths);
                    // run 'wtp command'
                    continue;
                }

                p.pop();
                println!("{}", p.display());
                paths.push(p);
            }
        });

    // println!("Paths: {:?}!", paths);
}

fn ask_wtp_list(path: &mut PathBuf, paths: &mut Vec<PathBuf>) {
    println!("Printing path: {:?}", path);
    let c = Command::new("wtp")
        .arg("list")
        .current_dir(path.as_path())
        .output()
        .expect("Failed calling WTP command");

    path.push("worktrees");

    String::from_utf8(c.stdout)
        .unwrap()
        .lines()
        .skip(2)
        .map(|l| l.split_once(' ').unwrap().0)
        .for_each(|l| {
            let mut tree = path.clone();
            tree.push(l);
            println!("{}", tree.display());
            paths.push(tree);
        });
}
