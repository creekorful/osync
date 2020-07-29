package index

import (
	"bufio"
	"crypto/sha1"
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
	"strings"
)

const (
	indexFile  = ".osync"
	ignoreFile = indexFile + "ignore"
)

type Index struct {
	directory string
	files     map[string]string
}

// Load try to load the directory index, returning a blank one if
// not index found
func Load(directory string) (Index, error) {
	indexPath := filepath.Join(directory, indexFile)
	if _, err := os.Stat(indexPath); os.IsNotExist(err) {
		// return blank index
		return Index{
			directory: directory,
			files:     map[string]string{},
		}, nil
	}

	index := Index{
		directory: directory,
		files:     map[string]string{},
	}

	// read the index file line by line
	lines, err := readLines(indexPath)
	if err != nil {
		return Index{}, err
	}

	for _, line := range lines {
		parts := strings.Split(line, ":")
		index.files[parts[0]] = parts[1]
	}

	return index, nil
}

// Compute the index for the given directory
func Compute(directory string) (Index, error) {
	ignoredFiles := map[string]bool{}

	// Try to load ignore file
	ignorePath := filepath.Join(directory, ignoreFile)
	if _, err := os.Stat(ignorePath); err == nil {
		lines, err := readLines(ignorePath)
		if err != nil {
			return Index{}, err
		}

		for _, line := range lines {
			ignoredFiles[line] = true
		}
	}

	index := Index{
		directory: directory,
		files:     map[string]string{},
	}

	if err := filepath.Walk(directory, func(path string, info os.FileInfo, err error) error {
		if info.Mode().IsRegular() {

			localPath := strings.TrimPrefix(path, directory+"/")

			// Skip file to ignore
			if _, exist := ignoredFiles[localPath]; exist {
				return nil
			}

			sha1, err := sha1sum(path)
			if err != nil {
				return err
			}

			index.files[localPath] = sha1
		}
		return nil
	}); err != nil {
		return Index{}, err
	}

	return index, nil
}

// Diff compute the difference between self and other
// returning the number of changes files (added, modified) and deleted files
func (i Index) Diff(other Index) ([]string, []string) {
	var changedFiles []string
	var deletedFiles []string

	for file, sum := range other.files {
		// if file is not in i.files it will return ""
		// the condition check: 'if this file is not in our index, or if the checksum has changed'
		if i.files[file] != sum {
			changedFiles = append(changedFiles, file)
		}
	}

	for file, _ := range i.files {
		if _, exist := other.files[file]; !exist {
			deletedFiles = append(deletedFiles, file)
		}
	}

	return changedFiles, deletedFiles
}

// Save current index to his directory
func (i Index) Save() error {
	file, err := os.Create(filepath.Join(i.directory, indexFile))
	if err != nil {
		return err
	}
	defer file.Close()

	w := bufio.NewWriter(file)
	defer w.Flush()

	for file, sum := range i.files {
		if _, err := w.WriteString(fmt.Sprintf("%s:%s\n", file, sum)); err != nil {
			return err
		}
	}

	return nil
}

func readLines(file string) ([]string, error) {
	var lines []string

	f, err := os.Open(file)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	sc := bufio.NewScanner(f)
	for sc.Scan() {
		lines = append(lines, sc.Text())
	}
	if err := sc.Err(); err != nil {
		return nil, err
	}

	return lines, err
}

func sha1sum(file string) (string, error) {
	f, err := os.Open(file)
	if err != nil {
		return "", err
	}
	defer f.Close()

	h := sha1.New()
	if _, err := io.Copy(h, f); err != nil {
		log.Fatal(err)
	}

	return fmt.Sprintf("%x", h.Sum(nil)), nil
}
