package com.example.service;

import java.util.List;
import java.util.Optional;

/**
 * Service for managing user accounts.
 */
public class UserService {
    private final UserRepository repository;

    public UserService(UserRepository repository) {
        this.repository = repository;
    }

    /** Find a user by their unique ID. */
    public Optional<User> findById(long id) {
        return repository.findById(id);
    }

    /** Create a new user account. */
    public User create(String name, String email) {
        User user = new User(name, email);
        return repository.save(user);
    }

    /** List all active users. */
    public List<User> listActive() {
        return repository.findByStatus("active");
    }
}

/** Represents a user account. */
class User {
    private final String name;
    private final String email;

    User(String name, String email) {
        this.name = name;
        this.email = email;
    }

    public String getName() { return name; }
    public String getEmail() { return email; }
}

/** Repository interface for user persistence. */
interface UserRepository {
    Optional<User> findById(long id);
    User save(User user);
    List<User> findByStatus(String status);
}
