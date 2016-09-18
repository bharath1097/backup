#!/usr/bin/perl
use strict;
use warnings;
use v5.10;

my @input = map { chomp; $_ } `cat ./data/file`;
my @add;
my @ignore;
for (@input) {
    next if /^#/;
    my($act, $file) = split / /, $_;
    ($file) = map { chomp; $_ } `readlink -f $file`;
    die "$file is invalid" unless (-f $file)||(-d $file);
    if ($act eq "+") {
        push @add, $file;
    } elsif ($act eq "-") {
        push @ignore, $file;
    } else {
        die "unknown action";
    }
}

my @files;
for (@add) {
#    if (-d $_.'/.git') {
#        my$prefix = $_;
#        push @files, map { chomp; $prefix.'/'.$_ } `cd $_ && git ls-files`;
#        push @files, "$_/.git";
#    } else {
#        push @files, map { chomp; $_ } `find $_`;
#        my @git = split /\n/, `find $_ -name .git`;
#        print "$_\n" for @git;
#    }
        push @files, map { chomp; $_ } `find $_`;
}

sub check {
    my($arg) = @_;
    for (@ignore) {
        return 0 if $arg eq $_;
        return 0 if $arg=~m/^\Q$_\/\E/;
    }
    return 1;
}
@files = sort grep { (-f $_) and &check($_) } @files;

system 'rm -R output' if -d 'output';
mkdir 'output';
chdir 'output';
for (@files) {
    s/^\///;
    my @dirs = split /\//;
    my$name = pop @dirs;
    my $current_dir = '';
    for (@dirs) {
        $current_dir.=$_.'/';
        mkdir $current_dir unless -d $current_dir;
    }
    system "ln -s \"/$_\" \"$current_dir\"";
}
