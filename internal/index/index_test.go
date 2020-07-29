package index

import (
	"io/ioutil"
	"os"
	"path/filepath"
	"testing"
)

func TestLoadNoIndex(t *testing.T) {
	dir := tempDir(t)
	defer os.RemoveAll(dir)

	index, err := Load(dir)
	if err != nil {
		t.Error(err)
	}

	if index.directory != dir {
		t.Error("Wrong directory")
	}
	if len(index.files) != 0 {
		t.Error("Invalid number of files")
	}
}

func TestLoadWithIndex(t *testing.T) {
	dir := tempDir(t)
	defer os.RemoveAll(dir)

	// Create dummy index
	if err := ioutil.WriteFile(filepath.Join(dir, indexFile), []byte("test:123445\nlol:1253425"), os.ModePerm); err != nil {
		t.Error(err)
	}

	index, err := Load(dir)
	if err != nil {
		t.Error(err)
	}

	if index.directory != dir {
		t.Error("Wrong directory")
	}
	if len(index.files) != 2 {
		t.Error("Invalid number of files")
	}
	if index.files["test"] != "123445" {
		t.Error("Wrong checksum for 'test'")
	}
	if index.files["lol"] != "1253425" {
		t.Error("Wrong checksum for 'lol'")
	}
}

func TestComputeNoFiles(t *testing.T) {
	dir := tempDir(t)
	defer os.RemoveAll(dir)

	index, err := Compute(dir)
	if err != nil {
		t.Error(err)
	}

	if index.directory != dir {
		t.Error("Wrong directory")
	}
	if len(index.files) != 0 {
		t.Error("Invalid number of files")
	}
}

func TestCompute(t *testing.T) {
	dir := tempDir(t)
	defer os.RemoveAll(dir)

	// Create some files
	if err := ioutil.WriteFile(filepath.Join(dir, "a"), []byte("a"), os.ModePerm); err != nil {
		t.Error(err)
	}
	if err := ioutil.WriteFile(filepath.Join(dir, "b"), []byte("b"), os.ModePerm); err != nil {
		t.Error(err)
	}

	// Ignore file b
	if err := ioutil.WriteFile(filepath.Join(dir, ignoreFile), []byte("b"), os.ModePerm); err != nil {
		t.Error(err)
	}

	index, err := Compute(dir)
	if err != nil {
		t.Error(err)
	}

	if index.directory != dir {
		t.Error("Wrong directory")
	}
	if len(index.files) != 2 {
		t.Error("Invalid number of files")
	}
	if index.files["a"] != "86f7e437faa5a7fce15d1ddcb9eaeaea377667b8" {
		t.Error("Wrong checksum for 'a'")
	}
}

func TestIndex_Diff(t *testing.T) {
	a := Index{
		files: map[string]string{
			"a": "a",
			"b": "b",
		},
	}

	b := Index{
		files: map[string]string{
			"a": "1",
			"c": "c",
		},
	}

	changedFiles, deletedFiles := a.Diff(b)

	// sum of a has changed
	if !contains(changedFiles, "a") {
		t.Error()
	}

	// c is a new file
	if !contains(changedFiles, "c") {
		t.Error()
	}

	// b has been removed
	if !contains(deletedFiles, "b") {
		t.Error()
	}
}

func TestIndex_Save(t *testing.T) {
	dir := tempDir(t)
	defer os.RemoveAll(dir)

	a := Index{
		files: map[string]string{
			"a": "a",
			"b": "b",
		},
		directory: dir,
	}

	if err := a.Save(); err != nil {
		t.Fatal(err)
	}

	b, err := ioutil.ReadFile(filepath.Join(dir, indexFile))
	if err != nil {
		t.Fatal(err)
	}
	if string(b) != "a:a\nb:b\n" {
		t.Error()
	}
}

func tempDir(t *testing.T) string {
	dir, err := ioutil.TempDir("", "osync")
	if err != nil {
		t.Fatal(err)
	}

	return dir
}

func contains(slice []string, elem string) bool {
	for _, val := range slice {
		if val == elem {
			return true
		}
	}

	return false
}
