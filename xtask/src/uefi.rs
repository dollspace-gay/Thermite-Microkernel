use std::collections::BTreeSet;

const PE_MACHINE_AMD64: u16 = 0x8664;
const PE_MAGIC_PE32_PLUS: u16 = 0x020b;
const PE_SUBSYSTEM_EFI_APPLICATION: u16 = 10;
const PE_DLL_DYNAMIC_BASE: u16 = 0x0040;
const PE_DLL_NX_COMPAT: u16 = 0x0100;
const FAT_VOLUME_ID: u32 = 0x544d_4b30;
const FAT_LABEL: &[u8; 11] = b"TMK_M0     ";

#[derive(Debug, Clone, Copy)]
pub struct PeAudit {
    pub text_file_offset: usize,
    pub text_size: usize,
}

#[derive(Debug)]
pub struct BootFile {
    pub bytes: Vec<u8>,
    pub first_data_offset: usize,
}

pub fn audit_pe(image: &[u8], expected_entry: &[u8]) -> Result<PeAudit, String> {
    if image.len() != 1024 {
        return Err(format!("PE image is {} bytes, expected 1024", image.len()));
    }
    require_slice(image, 0, 2, "DOS signature")?;
    if &image[0..2] != b"MZ" {
        return Err("PE image has no MZ signature".to_string());
    }
    let pe = u32_at(image, 0x3c, "PE header pointer")? as usize;
    if pe != 0x80 {
        return Err(format!("PE header offset is {pe:#x}, expected 0x80"));
    }
    require_slice(image, pe, 24, "PE/COFF header")?;
    if &image[pe..pe + 4] != b"PE\0\0" {
        return Err("PE image has no PE signature".to_string());
    }
    if u16_at(image, pe + 4, "COFF machine")? != PE_MACHINE_AMD64 {
        return Err("PE image machine is not x86_64".to_string());
    }
    if u16_at(image, pe + 6, "COFF section count")? != 1 {
        return Err("PE image must contain exactly one section".to_string());
    }
    if u32_at(image, pe + 8, "COFF timestamp")? != 0 {
        return Err("PE COFF timestamp is not zero".to_string());
    }
    if u32_at(image, pe + 12, "COFF symbol table pointer")? != 0
        || u32_at(image, pe + 16, "COFF symbol count")? != 0
    {
        return Err("PE image retains a COFF symbol table".to_string());
    }
    let optional_size = u16_at(image, pe + 20, "optional-header size")? as usize;
    if optional_size != 0xf0 {
        return Err(format!(
            "PE optional-header size is {optional_size:#x}, expected 0xf0"
        ));
    }
    let optional = pe + 24;
    require_slice(image, optional, optional_size, "PE optional header")?;
    if u16_at(image, optional, "PE optional-header magic")? != PE_MAGIC_PE32_PLUS {
        return Err("PE image is not PE32+".to_string());
    }
    if u32_at(image, optional + 16, "entry RVA")? != 0x1000 {
        return Err("PE entry RVA is not 0x1000".to_string());
    }
    if u64_at(image, optional + 24, "image base")? != 0x10_0000 {
        return Err("PE image base is not 0x100000".to_string());
    }
    if u32_at(image, optional + 32, "section alignment")? != 4096
        || u32_at(image, optional + 36, "file alignment")? != 512
    {
        return Err("PE section/file alignment is not 4096/512".to_string());
    }
    if u32_at(image, optional + 56, "image size")? != 0x2000
        || u32_at(image, optional + 60, "header size")? != 0x200
    {
        return Err("PE image/header size is not 0x2000/0x200".to_string());
    }
    if u16_at(image, optional + 68, "PE subsystem")? != PE_SUBSYSTEM_EFI_APPLICATION {
        return Err("PE subsystem is not EFI application".to_string());
    }
    let dll = u16_at(image, optional + 70, "PE DLL characteristics")?;
    if dll & PE_DLL_NX_COMPAT == 0 || dll & PE_DLL_DYNAMIC_BASE != 0 {
        return Err(format!(
            "PE DLL characteristics {dll:#06x} do not require NX and fixed image base"
        ));
    }
    if u32_at(image, optional + 108, "PE data-directory count")? != 16 {
        return Err("PE image does not declare exactly 16 data directories".to_string());
    }
    require_slice(image, optional + 112, 16 * 8, "PE data directories")?;
    if image[optional + 112..optional + 112 + 16 * 8]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err("PE image contains a nonempty data directory".to_string());
    }

    let section = optional + optional_size;
    require_slice(image, section, 40, "PE section header")?;
    if &image[section..section + 8] != b".text\0\0\0" {
        return Err("PE image's only section is not .text".to_string());
    }
    if u32_at(image, section + 8, ".text virtual size")? as usize != expected_entry.len()
        || u32_at(image, section + 12, ".text RVA")? != 0x1000
        || u32_at(image, section + 16, ".text raw size")? != 512
        || u32_at(image, section + 20, ".text raw offset")? != 512
    {
        return Err("PE .text layout does not match the registered entry capsule".to_string());
    }
    if u32_at(image, section + 24, ".text relocation pointer")? != 0
        || u16_at(image, section + 32, ".text relocation count")? != 0
    {
        return Err("PE .text retains relocations".to_string());
    }
    let characteristics = u32_at(image, section + 36, ".text characteristics")?;
    let required = 0x0000_0020 | 0x2000_0000 | 0x4000_0000;
    if characteristics & required != required || characteristics & 0x8000_0000 != 0 {
        return Err(format!(
            "PE .text characteristics {characteristics:#010x} are not RX code"
        ));
    }
    let text_offset = 512usize;
    require_slice(image, text_offset, 512, "PE .text raw data")?;
    if &image[text_offset..text_offset + expected_entry.len()] != expected_entry {
        return Err("PE .text bytes differ from the verified entry capsule".to_string());
    }
    if image[text_offset + expected_entry.len()..text_offset + 512]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err("PE .text file padding is not zero".to_string());
    }
    Ok(PeAudit {
        text_file_offset: text_offset,
        text_size: expected_entry.len(),
    })
}

pub fn extract_bootx64(image: &[u8]) -> Result<BootFile, String> {
    if image.len() != 32 * 1024 * 1024 {
        return Err(format!(
            "FAT image is {} bytes, expected 33554432",
            image.len()
        ));
    }
    require_slice(image, 0, 512, "FAT boot sector")?;
    if image[510..512] != [0x55, 0xaa] {
        return Err("FAT boot-sector signature is invalid".to_string());
    }
    let bytes_per_sector = u16_at(image, 11, "FAT bytes per sector")? as usize;
    let sectors_per_cluster = image[13] as usize;
    let reserved = u16_at(image, 14, "FAT reserved sectors")? as usize;
    let fat_count = image[16] as usize;
    let root_entries = u16_at(image, 17, "FAT root entries")? as usize;
    let total16 = u16_at(image, 19, "FAT total sectors (16-bit)")? as usize;
    let sectors_per_fat = u16_at(image, 22, "FAT sectors per FAT")? as usize;
    let total32 = u32_at(image, 32, "FAT total sectors (32-bit)")? as usize;
    let total_sectors = if total16 != 0 { total16 } else { total32 };
    if bytes_per_sector != 512
        || sectors_per_cluster != 4
        || reserved != 4
        || fat_count != 2
        || root_entries != 512
        || sectors_per_fat != 64
        || total_sectors != 65536
        || total_sectors * bytes_per_sector != image.len()
    {
        return Err("FAT16 BPB geometry is not the fixed M0 geometry".to_string());
    }
    if image[21] != 0xf8
        || u16_at(image, 24, "FAT sectors per track")? != 32
        || u16_at(image, 26, "FAT heads")? != 4
        || u32_at(image, 28, "FAT hidden sectors")? != 0
    {
        return Err("FAT16 media geometry is not the fixed M0 geometry".to_string());
    }
    if u32_at(image, 39, "FAT volume ID")? != FAT_VOLUME_ID
        || &image[43..54] != FAT_LABEL
        || &image[54..62] != b"FAT16   "
    {
        return Err("FAT16 identity fields are not the fixed M0 values".to_string());
    }
    let root_sectors = (root_entries * 32).div_ceil(bytes_per_sector);
    let fat_start = checked_mul(reserved, bytes_per_sector, "FAT offset")?;
    let root_sector = reserved
        .checked_add(checked_mul(fat_count, sectors_per_fat, "FAT span")?)
        .ok_or_else(|| "FAT root-sector overflow".to_string())?;
    let root_start = checked_mul(root_sector, bytes_per_sector, "FAT root offset")?;
    let data_sector = root_sector
        .checked_add(root_sectors)
        .ok_or_else(|| "FAT data-sector overflow".to_string())?;
    let data_clusters = (total_sectors - data_sector) / sectors_per_cluster;
    if !(4085..65525).contains(&data_clusters) {
        return Err("FAT BPB does not describe a FAT16 cluster count".to_string());
    }
    let data_start = checked_mul(data_sector, bytes_per_sector, "FAT data offset")?;
    let fat_bytes = checked_mul(sectors_per_fat, bytes_per_sector, "FAT byte span")?;
    require_slice(image, fat_start, fat_bytes * fat_count, "FAT table")?;
    if image[fat_start..fat_start + fat_bytes]
        != image[fat_start + fat_bytes..fat_start + fat_bytes * 2]
    {
        return Err("FAT16 table mirrors differ".to_string());
    }
    if u16_at(image, fat_start, "FAT16 media entry")? != 0xfff8
        || u16_at(image, fat_start + 2, "FAT16 reserved entry")? < 0xfff8
    {
        return Err("FAT16 reserved entries are invalid".to_string());
    }
    require_slice(image, root_start, root_entries * 32, "FAT root directory")?;

    let root = directory_entries(&image[root_start..root_start + root_entries * 32], "root")?;
    find_exact(&root, b"TMK_M0     ", 0x08, "volume label")?;
    let efi = find_exact(&root, b"EFI        ", 0x10, "EFI directory")?;
    audit_fixed_time(efi, "EFI directory")?;
    require_names(&root, &[b"TMK_M0     ", b"EFI        "], "root")?;

    let efi_bytes = read_directory_cluster(
        image,
        efi.cluster,
        fat_start,
        data_start,
        bytes_per_sector,
        sectors_per_cluster,
        "EFI directory",
    )?;
    let efi_entries = directory_entries(efi_bytes, "EFI directory")?;
    let boot = find_exact(&efi_entries, b"BOOT       ", 0x10, "BOOT directory")?;
    audit_fixed_time(boot, "BOOT directory")?;
    require_names(
        &efi_entries,
        &[b".          ", b"..         ", b"BOOT       "],
        "EFI directory",
    )?;

    let boot_bytes = read_directory_cluster(
        image,
        boot.cluster,
        fat_start,
        data_start,
        bytes_per_sector,
        sectors_per_cluster,
        "BOOT directory",
    )?;
    let boot_entries = directory_entries(boot_bytes, "BOOT directory")?;
    let file = find_exact(&boot_entries, b"BOOTX64 EFI", 0x20, "BOOTX64.EFI")?;
    audit_fixed_time(file, "BOOTX64.EFI")?;
    require_names(
        &boot_entries,
        &[b".          ", b"..         ", b"BOOTX64 EFI"],
        "BOOT directory",
    )?;
    if file.size == 0 || file.cluster < 2 {
        return Err("BOOTX64.EFI has an empty or invalid cluster chain".to_string());
    }
    let cluster_size = checked_mul(bytes_per_sector, sectors_per_cluster, "FAT cluster size")?;
    let first_data_offset = cluster_offset(data_start, cluster_size, file.cluster)?;
    let mut cluster = file.cluster;
    let mut seen = BTreeSet::new();
    let mut bytes = Vec::with_capacity(file.size);
    while bytes.len() < file.size {
        if !seen.insert(cluster) {
            return Err("BOOTX64.EFI cluster chain contains a loop".to_string());
        }
        let offset = cluster_offset(data_start, cluster_size, cluster)?;
        require_slice(image, offset, cluster_size, "BOOTX64.EFI cluster")?;
        let remaining = file.size - bytes.len();
        bytes.extend_from_slice(&image[offset..offset + remaining.min(cluster_size)]);
        let next = u16_at(
            image,
            fat_start + cluster as usize * 2,
            "BOOTX64.EFI FAT entry",
        )?;
        if bytes.len() == file.size {
            if next < 0xfff8 {
                return Err("BOOTX64.EFI cluster chain has unneeded trailing clusters".to_string());
            }
        } else if !(2..0xfff8).contains(&next) || next == 0xfff7 {
            return Err("BOOTX64.EFI cluster chain terminates early".to_string());
        } else {
            cluster = next;
        }
    }
    Ok(BootFile {
        bytes,
        first_data_offset,
    })
}

#[derive(Debug)]
struct DirEntry<'a> {
    name: &'a [u8],
    attr: u8,
    cluster: u16,
    size: usize,
    raw: &'a [u8],
}

fn directory_entries<'a>(bytes: &'a [u8], label: &str) -> Result<Vec<DirEntry<'a>>, String> {
    if !bytes.len().is_multiple_of(32) {
        return Err(format!(
            "{label} byte length is not directory-entry aligned"
        ));
    }
    let mut entries = Vec::new();
    for raw in bytes.chunks_exact(32) {
        if raw[0] == 0 {
            break;
        }
        if raw[0] == 0xe5 {
            return Err(format!("{label} contains a deleted directory entry"));
        }
        if raw[11] == 0x0f {
            return Err(format!("{label} contains a forbidden long-filename entry"));
        }
        let high = u16::from_le_bytes([raw[20], raw[21]]);
        if high != 0 {
            return Err(format!("{label} uses a nonzero FAT32 cluster high word"));
        }
        entries.push(DirEntry {
            name: &raw[..11],
            attr: raw[11],
            cluster: u16::from_le_bytes([raw[26], raw[27]]),
            size: u32::from_le_bytes([raw[28], raw[29], raw[30], raw[31]]) as usize,
            raw,
        });
    }
    Ok(entries)
}

fn find_exact<'a>(
    entries: &'a [DirEntry<'a>],
    name: &[u8; 11],
    attr: u8,
    label: &str,
) -> Result<&'a DirEntry<'a>, String> {
    let matches: Vec<_> = entries
        .iter()
        .filter(|entry| entry.name == name && entry.attr == attr)
        .collect();
    if matches.len() != 1 {
        return Err(format!("{label} occurs {} times", matches.len()));
    }
    Ok(matches[0])
}

fn require_names(
    entries: &[DirEntry<'_>],
    expected: &[&[u8; 11]],
    label: &str,
) -> Result<(), String> {
    let actual: Vec<Vec<u8>> = entries.iter().map(|entry| entry.name.to_vec()).collect();
    let wanted: Vec<Vec<u8>> = expected.iter().map(|name| name.to_vec()).collect();
    if actual != wanted {
        return Err(format!(
            "{label} entries are not the fixed canonical set/order"
        ));
    }
    Ok(())
}

fn audit_fixed_time(entry: &DirEntry<'_>, label: &str) -> Result<(), String> {
    let create_time = u16::from_le_bytes([entry.raw[14], entry.raw[15]]);
    let create_date = u16::from_le_bytes([entry.raw[16], entry.raw[17]]);
    let access_date = u16::from_le_bytes([entry.raw[18], entry.raw[19]]);
    let write_time = u16::from_le_bytes([entry.raw[22], entry.raw[23]]);
    let write_date = u16::from_le_bytes([entry.raw[24], entry.raw[25]]);
    if create_time != 0 || write_time != 0 {
        return Err(format!("{label} has a nonzero FAT time"));
    }
    if create_date != 0x0021 || access_date != 0x0021 || write_date != 0x0021 {
        return Err(format!("{label} does not use the fixed 1980-01-01 date"));
    }
    Ok(())
}

fn read_directory_cluster<'a>(
    image: &'a [u8],
    cluster: u16,
    fat_start: usize,
    data_start: usize,
    bytes_per_sector: usize,
    sectors_per_cluster: usize,
    label: &str,
) -> Result<&'a [u8], String> {
    if cluster < 2 {
        return Err(format!("{label} has invalid cluster {cluster}"));
    }
    let cluster_size = checked_mul(bytes_per_sector, sectors_per_cluster, "cluster size")?;
    let offset = cluster_offset(data_start, cluster_size, cluster)?;
    require_slice(image, offset, cluster_size, label)?;
    let next = u16_at(image, fat_start + cluster as usize * 2, label)?;
    if next < 0xfff8 {
        return Err(format!("{label} must fit in one cluster"));
    }
    Ok(&image[offset..offset + cluster_size])
}

fn cluster_offset(data_start: usize, cluster_size: usize, cluster: u16) -> Result<usize, String> {
    data_start
        .checked_add(checked_mul(
            cluster as usize - 2,
            cluster_size,
            "cluster offset",
        )?)
        .ok_or_else(|| "cluster offset overflow".to_string())
}

fn checked_mul(left: usize, right: usize, label: &str) -> Result<usize, String> {
    left.checked_mul(right)
        .ok_or_else(|| format!("{label} overflow"))
}

fn require_slice(bytes: &[u8], offset: usize, length: usize, label: &str) -> Result<(), String> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| format!("{label} range overflow"))?;
    if end > bytes.len() {
        Err(format!("{label} extends beyond its containing image"))
    } else {
        Ok(())
    }
}

fn u16_at(bytes: &[u8], offset: usize, label: &str) -> Result<u16, String> {
    require_slice(bytes, offset, 2, label)?;
    Ok(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
}

fn u32_at(bytes: &[u8], offset: usize, label: &str) -> Result<u32, String> {
    require_slice(bytes, offset, 4, label)?;
    Ok(u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}

fn u64_at(bytes: &[u8], offset: usize, label: &str) -> Result<u64, String> {
    require_slice(bytes, offset, 8, label)?;
    Ok(u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ]))
}
