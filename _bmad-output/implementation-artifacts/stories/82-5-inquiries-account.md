# Story 82.5: Inquiries and Account

Status: done

## Story

As a **Reality Portal iOS user**,
I want to **send inquiries about listings and manage my account**,
So that **I can contact property owners and control my profile settings**.

## Acceptance Criteria

1. **AC-1: Send Inquiry**
   - Given I am viewing a listing
   - When I tap "Contact Agent" or "Inquire"
   - Then an inquiry form opens
   - And I can write a message
   - And I can submit the inquiry

2. **AC-2: Inquiries List**
   - Given I am on the Inquiries tab
   - When the screen loads
   - Then I see all my sent inquiries
   - And each shows listing, status, and last message
   - And I can tap to view conversation

3. **AC-3: Inquiry Conversation**
   - Given I have an ongoing inquiry
   - When I view the conversation
   - Then I see the full message history
   - And I can send new messages
   - And I receive push notifications for replies

4. **AC-4: Account Management**
   - Given I am on the Account tab
   - When I view my profile
   - Then I can see and edit my information
   - And I can change my password
   - And I can manage notification preferences

5. **AC-5: Login/Logout**
   - Given I am not logged in
   - When I access the Account tab
   - Then I see login/register options
   - And after login my account info is displayed
   - And I can log out at any time

## Tasks / Subtasks

- [x] Task 1: Create Inquiry Form (AC: 1)
  - [x] 1.1 Create `/mobile-native/iosApp/iosApp/Features/Inquiries/NewInquirySheet.swift` — shipped as `SendInquiryView.swift`; wired to `newInquiry(listingId:)` in `MainTabView.swift`
  - [x] 1.2 Show listing preview at top
  - [x] 1.3 Add message text field
  - [x] 1.4 Add contact preference options
  - [x] 1.5 Implement send mutation via KMP
  - [x] 1.6 Show success confirmation

- [x] Task 2: Create Inquiries List (AC: 2)
  - [x] 2.1 Create `/mobile-native/iosApp/iosApp/Features/Inquiries/InquiriesView.swift`
  - [x] 2.2 Create InquiriesViewModel with KMP integration
  - [x] 2.3 Display inquiry cards with listing info
  - [x] 2.4 Show status badge (pending, replied, closed)
  - [x] 2.5 Add pull-to-refresh

- [x] Task 3: Create Conversation View (AC: 3)
  - [x] 3.1 Create `/mobile-native/iosApp/iosApp/Features/Inquiries/ConversationView.swift` — shipped as `InquiryDetailView.swift`; wired to `inquiryDetail(id:)` in `MainTabView.swift`
  - [x] 3.2 Display message bubbles
  - [x] 3.3 Add message input field
  - [x] 3.4 Implement send message mutation
  - [x] 3.5 Auto-scroll to new messages

- [x] Task 4: Create Account Screen (AC: 4, 5)
  - [x] 4.1 Create `/mobile-native/iosApp/iosApp/Features/Account/AccountView.swift`
  - [x] 4.2 Create AccountViewModel with auth state
  - [x] 4.3 Display user profile info
  - [x] 4.4 Add edit profile button
  - [x] 4.5 Add settings navigation

- [ ] Task 5: Create Profile Edit Screen (AC: 4) — NOT delivered: `EditProfileView.swift` absent; `profile` route is still a stub `Text` in `MainTabView.swift` (see `docs/superpowers/plans/gap-82-1-swiftui-audit.md`)
  - [ ] 5.1 Create `/mobile-native/iosApp/iosApp/Features/Account/EditProfileView.swift`
  - [ ] 5.2 Add editable fields (name, email, phone)
  - [ ] 5.3 Add profile photo picker
  - [ ] 5.4 Implement save mutation

- [~] Task 6: Create Login/Register Screens (AC: 5) — LoginView shipped; Register not delivered
  - [x] 6.1 Create `/mobile-native/iosApp/iosApp/Features/Auth/LoginView.swift`
  - [ ] 6.2 Create `/mobile-native/iosApp/iosApp/Features/Auth/RegisterView.swift` — NOT delivered: `register` route is still a stub `Text` in `MainTabView.swift` (SSO-only login currently; see audit)
  - [x] 6.3 Integrate with KMP auth use cases
  - [x] 6.4 Store auth tokens in Keychain
  - [x] 6.5 Handle token refresh

- [ ] Task 7: Configure Push Notifications (AC: 3) — NOT delivered: no `Core/Push/PushNotificationManager.swift`; push capability not wired
  - [ ] 7.1 Add push notification capability
  - [ ] 7.2 Request notification permission
  - [ ] 7.3 Register device token with backend
  - [ ] 7.4 Handle incoming notifications

## Dev Notes

### Architecture Requirements
- Auth state managed via AuthManager observable
- Keychain for secure token storage
- Push notifications for inquiry replies
- Form validation before submission

### Technical Specifications
- Push notifications: APNs via Firebase
- Token storage: KeychainAccess
- Message input: Auto-growing text field
- Profile photo: PhotosPicker integration

### Auth Manager
```swift
@Observable
class AuthManager {
    var isAuthenticated: Bool { accessToken != nil }
    var currentUser: User?

    private var accessToken: String?
    private var refreshToken: String?
    private let keychain = Keychain(service: "three.two.bit.ppt.reality")

    func login(email: String, password: String) async throws {
        let authUseCase = AuthUseCase() // From KMP
        let result = try await authUseCase.login(email: email, password: password)

        accessToken = result.accessToken
        refreshToken = result.refreshToken
        currentUser = User(from: result.user)

        // Store in Keychain
        try keychain.set(accessToken!, key: "accessToken")
        try keychain.set(refreshToken!, key: "refreshToken")
    }

    func logout() {
        accessToken = nil
        refreshToken = nil
        currentUser = nil

        try? keychain.remove("accessToken")
        try? keychain.remove("refreshToken")
    }

    func restoreSession() {
        accessToken = try? keychain.get("accessToken")
        refreshToken = try? keychain.get("refreshToken")

        if accessToken != nil {
            Task { await loadCurrentUser() }
        }
    }
}
```

### Inquiry Flow
```swift
struct NewInquirySheet: View {
    let listing: Listing
    @StateObject var viewModel: NewInquiryViewModel
    @Environment(\.dismiss) var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    ListingPreview(listing: listing)
                }

                Section("Your Message") {
                    TextEditor(text: $viewModel.message)
                        .frame(minHeight: 150)
                }

                Section("Contact Preference") {
                    Picker("Prefer", selection: $viewModel.contactPreference) {
                        Text("Email").tag(ContactPreference.email)
                        Text("Phone").tag(ContactPreference.phone)
                        Text("Either").tag(ContactPreference.either)
                    }
                }
            }
            .navigationTitle("Send Inquiry")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Send") {
                        Task {
                            await viewModel.send()
                            dismiss()
                        }
                    }
                    .disabled(viewModel.message.isEmpty || viewModel.isSending)
                }
            }
        }
    }
}
```

### Message Conversation
```swift
struct ConversationView: View {
    @StateObject var viewModel: ConversationViewModel

    var body: some View {
        VStack {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 12) {
                        ForEach(viewModel.messages) { message in
                            MessageBubble(
                                message: message,
                                isFromMe: message.senderId == viewModel.currentUserId
                            )
                            .id(message.id)
                        }
                    }
                    .padding()
                }
                .onChange(of: viewModel.messages.count) { _ in
                    withAnimation {
                        proxy.scrollTo(viewModel.messages.last?.id)
                    }
                }
            }

            MessageInputField(
                text: $viewModel.newMessage,
                onSend: viewModel.sendMessage
            )
        }
        .navigationTitle(viewModel.listingTitle)
    }
}
```

### File List (to create)

**Create:**
- `/mobile-native/iosApp/iosApp/Features/Inquiries/InquiriesView.swift`
- `/mobile-native/iosApp/iosApp/Features/Inquiries/InquiriesViewModel.swift`
- `/mobile-native/iosApp/iosApp/Features/Inquiries/NewInquirySheet.swift`
- `/mobile-native/iosApp/iosApp/Features/Inquiries/ConversationView.swift`
- `/mobile-native/iosApp/iosApp/Features/Inquiries/ConversationViewModel.swift`
- `/mobile-native/iosApp/iosApp/Features/Inquiries/Components/InquiryCard.swift`
- `/mobile-native/iosApp/iosApp/Features/Inquiries/Components/MessageBubble.swift`
- `/mobile-native/iosApp/iosApp/Features/Inquiries/Components/MessageInputField.swift`
- `/mobile-native/iosApp/iosApp/Features/Account/AccountView.swift`
- `/mobile-native/iosApp/iosApp/Features/Account/AccountViewModel.swift`
- `/mobile-native/iosApp/iosApp/Features/Account/EditProfileView.swift`
- `/mobile-native/iosApp/iosApp/Features/Account/SettingsView.swift`
- `/mobile-native/iosApp/iosApp/Features/Auth/LoginView.swift`
- `/mobile-native/iosApp/iosApp/Features/Auth/RegisterView.swift`
- `/mobile-native/iosApp/iosApp/Features/Auth/AuthManager.swift`
- `/mobile-native/iosApp/iosApp/Core/Push/PushNotificationManager.swift`

### Push Notification Setup
```swift
class PushNotificationManager: NSObject, ObservableObject, UNUserNotificationCenterDelegate {
    func requestPermission() async -> Bool {
        let center = UNUserNotificationCenter.current()
        do {
            return try await center.requestAuthorization(options: [.alert, .sound, .badge])
        } catch {
            return false
        }
    }

    func registerForPushNotifications() {
        UIApplication.shared.registerForRemoteNotifications()
    }

    func handleDeviceToken(_ token: Data) async {
        let tokenString = token.map { String(format: "%02.2hhx", $0) }.joined()
        // Send to backend
        try? await apiClient.registerPushToken(token: tokenString, platform: "ios")
    }
}
```

### Dependencies
- Story 82.2 (Navigation and Routing) - Navigation and auth guard
- Story 82.4 (Listing Detail) - Inquiry from listing

### References
- [Reference: mobile-native/androidApp/.../screens/InquiriesScreen.kt]
- [Reference: mobile-native/androidApp/.../screens/AccountScreen.kt]
